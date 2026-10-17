//! `room-cli mac pull` and `push`. Up to 16 MiB still travels in one Mac job, as before. Bigger copies, up to
//! 2 GiB, go in 8 MiB pieces through the same jobs: staged in a Mac temp folder, then checked with SHA-256.
//! Every piece of a copy runs the same command, with its number in stdin, so the Mac helper's Jev check
//! (ADR 0224) reviews one command per copy, not one per piece.
use super::{LIMIT, finished, hex, quote, request};
use serde_json::json;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const MIB: u64 = 1024 * 1024;
/// The AWS CLI's multipart default, well inside one job's 16 MiB output.
const PIECE: u64 = 8 * MIB;
const MAX: u64 = 2048 * MIB;
const TRIES: u32 = 3;
/// The pack job's exit code when its archive is too big for one job and waits in a Mac temp folder.
const STAGED: i32 = 75;
/// SHA-256 of stdin on macOS and Linux.
const SHA: &str = "{ sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-64";
/// Read-only Mac access may write in /tmp.
const MKTEMP: &str = "mktemp -d /tmp/cloudroom.XXXXXX";

pub(super) async fn pull(socket: &str, source: &str, target: &str) -> io::Result<i32> {
    // `head` stops the archive one byte past the limit, so an oversized pull never fills the Mac's disk.
    let pack = format!(
        "p={}; t=$({MKTEMP}) || exit 1; cd -- \"$(dirname -- \"$p\")\" || {{ rm -rf \"$t\"; exit 1; }}; \
         {{ COPYFILE_DISABLE=1 tar --no-xattrs -czf - -- \"$(basename -- \"$p\")\" || touch \"$t/failed\"; }} \
         | head -c {} > \"$t/a.tgz\"; s=$(wc -c < \"$t/a.tgz\" | tr -d ' '); \
         if [ \"$s\" -gt {MAX} ]; then rm -rf \"$t\"; echo 'The packed files are over the 2 GiB limit' >&2; exit 1; fi; \
         if [ -e \"$t/failed\" ]; then rm -rf \"$t\"; exit 1; fi; \
         if [ \"$s\" -le {LIMIT} ]; then cat \"$t/a.tgz\"; rm -rf \"$t\"; exit 0; fi; \
         printf '%s\\n%s\\n%s' \"$t\" \"$s\" \"$(cat \"$t/a.tgz\" | {SHA})\"; exit {STAGED}",
        quote(source),
        MAX + 1
    );
    let archive = Temp::new("pull");
    match run(socket, pack, &[], false).await? {
        (Some(0), stdout, _) => fs::write(&archive.0, stdout)?,
        (Some(STAGED), stdout, _) => {
            let text = String::from_utf8_lossy(&stdout);
            let &[dir, size, sha] = &text.split('\n').collect::<Vec<_>>()[..] else {
                return Err(io::Error::other("invalid Mac archive"));
            };
            let dir = temp_dir(dir)?;
            let fetched = fetch(socket, &dir, size, sha, &archive.0).await;
            let _ = job(socket, format!("rm -rf {dir}"), &[], false).await;
            fetched?;
        }
        (_, _, stderr) => {
            return Err(io::Error::other(
                String::from_utf8_lossy(&stderr).into_owned(),
            ));
        }
    }
    let target = Path::new(target);
    fs::create_dir_all(target)?;
    let unpacked = tokio::process::Command::new("tar")
        .arg("-xzf")
        .arg(&archive.0)
        .arg("-C")
        .arg(target)
        .status()
        .await?;
    if !unpacked.success() {
        return Err(io::Error::other("Could not unpack the copied files"));
    }
    println!("{}", json!({"copied":source,"to":target}));
    Ok(0)
}

/// Copies a staged Mac archive into `path` piece by piece, then checks its SHA-256.
async fn fetch(socket: &str, dir: &str, size: &str, sha: &str, path: &Path) -> io::Result<()> {
    let size: u64 = size
        .parse()
        .map_err(|_| io::Error::other("invalid Mac archive size"))?;
    if size > MAX {
        return Err(io::Error::other(
            "The packed files are over the 2 GiB limit",
        ));
    }
    let mut file = fs::File::create(path)?;
    let pieces = size.div_ceil(PIECE);
    let command = format!(
        "dd if={dir}/a.tgz bs={MIB} count={} skip=$(cat)",
        PIECE / MIB
    );
    for i in 0..pieces {
        let skip = (i * (PIECE / MIB)).to_string();
        let want = PIECE.min(size - i * PIECE);
        let bytes = piece(socket, &command, skip.as_bytes(), Some(want)).await?;
        file.write_all(&bytes)?;
        progress(i, pieces);
    }
    verify(path, sha).await
}

pub(super) async fn push(socket: &str, source: &str, target: &str) -> io::Result<i32> {
    let path = Path::new(source);
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::other("push needs a file or folder path"));
    };
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let archive = Temp::new("push");
    let packed = tokio::process::Command::new("tar")
        .arg("-czf")
        .arg(&archive.0)
        .arg("-C")
        .arg(parent)
        .arg("--")
        .arg(name)
        .status()
        .await?;
    let size = fs::metadata(&archive.0).map_or(u64::MAX, |m| m.len());
    if !packed.success() || size > MAX {
        return Err(io::Error::other(
            "Could not pack the files; the compressed limit is 2 GiB",
        ));
    }
    let unpack = format!("d={}; mkdir -p -- \"$d\" && ", quote(target));
    if size <= LIMIT as u64 {
        let input = fs::read(&archive.0)?;
        job(
            socket,
            format!("{unpack}tar -xzf - -C \"$d\""),
            &input,
            true,
        )
        .await?;
    } else {
        let dir = temp_dir(&String::from_utf8_lossy(
            &job(socket, MKTEMP.into(), &[], false).await?,
        ))?;
        let clean = || job(socket, format!("rm -rf {dir}"), &[], false);
        if let Err(error) = stage(socket, &dir, &archive.0, size).await {
            let _ = clean().await;
            return Err(error);
        }
        // Only the unpack changes the Mac, so Ask first asks once. It removes the staging itself, and an
        // unknown outcome (`?`) leaves it alone, so cleanup never deletes files an unpack is still reading.
        let unpack =
            format!("{unpack}cat {dir}/* | tar -xzf - -C \"$d\"; s=$?; rm -rf {dir}; exit $s");
        if let (code, _, stderr) = run(socket, unpack, &[], true).await?
            && code != Some(0)
        {
            let _ = clean().await; // Refused or denied: the unpack never ran.
            return Err(io::Error::other(
                String::from_utf8_lossy(&stderr).into_owned(),
            ));
        }
    }
    println!("{}", json!({"copied":source,"to":target}));
    Ok(0)
}

/// Stages `path` in a Mac temp folder piece by piece and checks its SHA-256 there, all read-only.
async fn stage(socket: &str, dir: &str, path: &Path, size: u64) -> io::Result<()> {
    let file = fs::File::open(path)?;
    let pieces = size.div_ceil(PIECE);
    // Each piece lands under a hidden name, then moves into place whole: a late copy of a retried piece
    // can only swap in identical bytes, never half-rewrite one.
    let command = format!(
        "n=$(dd bs=1 count=5 2>/dev/null) && cat > {dir}/.$n.$$ && mv -f {dir}/.$n.$$ {dir}/$n"
    );
    for i in 0..pieces {
        let mut input = format!("{i:05}").into_bytes();
        (&file).take(PIECE).read_to_end(&mut input)?;
        piece(socket, &command, &input, None).await?;
        progress(i, pieces);
    }
    let sha = job(socket, format!("cat {dir}/* | {SHA}"), &[], false).await?;
    verify(path, &String::from_utf8_lossy(&sha)).await
}

/// Runs one Mac command: its exit code, stdout, and stderr.
async fn run(
    socket: &str,
    command: String,
    stdin: &[u8],
    write: bool,
) -> io::Result<(Option<i32>, Vec<u8>, Vec<u8>)> {
    let body = json!({"command":command,"stdin":hex(stdin),"write":write});
    let value = request(socket, "POST", "/jobs", Some(body)).await?;
    if let Some(id) = value["job"].as_str().filter(|_| value["state"] == "done") {
        let _ = request(socket, "DELETE", &format!("/jobs/{id}"), None).await;
    }
    finished(value)
}

/// Runs one Mac command and returns its stdout, or an error with its stderr.
async fn job(socket: &str, command: String, stdin: &[u8], write: bool) -> io::Result<Vec<u8>> {
    match run(socket, command, stdin, write).await? {
        (Some(0), stdout, _) => Ok(stdout),
        (_, _, stderr) => Err(io::Error::other(
            String::from_utf8_lossy(&stderr).into_owned(),
        )),
    }
}

/// One piece, tried up to three times. A piece only reads or overwrites its own bytes, so repeating it is safe.
/// A pulled piece must bring exactly `want` bytes; a pushed piece's output is ignored.
async fn piece(
    socket: &str,
    command: &str,
    stdin: &[u8],
    want: Option<u64>,
) -> io::Result<Vec<u8>> {
    let mut tries = 1;
    loop {
        let error = match (job(socket, command.to_owned(), stdin, false).await, want) {
            (Ok(bytes), Some(want)) if bytes.len() as u64 != want => io::Error::other(format!(
                "a piece arrived with {} of {want} bytes",
                bytes.len()
            )),
            (Ok(bytes), _) => return Ok(bytes),
            (Err(error), _) => error,
        };
        if tries == TRIES {
            return Err(error);
        }
        eprintln!("Retrying a piece: {error}");
        tries += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn progress(i: u64, pieces: u64) {
    if i % 8 == 7 || i + 1 == pieces {
        eprintln!("Copied {} of {pieces} pieces", i + 1);
    }
}

/// The last line a Mac command printed: a chatty login shell may print before it.
fn last_line(text: &str) -> &str {
    text.trim().lines().last().unwrap_or_default()
}

/// The Mac temp folder from `MKTEMP`, checked before any `rm -rf`, so cleanup never removes anything else.
fn temp_dir(text: &str) -> io::Result<String> {
    let dir = last_line(text);
    match dir.strip_prefix("/tmp/cloudroom.") {
        Some(name) if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric()) => {
            Ok(dir.to_owned())
        }
        _ => Err(io::Error::other("The Mac temp folder is unavailable")),
    }
}

/// Fails unless the local file's SHA-256 matches the Mac's.
async fn verify(path: &Path, mac: &str) -> io::Result<()> {
    let out = tokio::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .await?;
    let local = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && local.get(..64) == Some(last_line(mac)) {
        return Ok(());
    }
    Err(io::Error::other(
        "The copy was damaged on the way (SHA-256 mismatch); try again",
    ))
}

/// A local temp archive, removed when dropped.
struct Temp(PathBuf);
impl Temp {
    fn new(kind: &str) -> Self {
        let name = format!("cloudroom-{kind}-{}.tgz", std::process::id());
        Self(std::env::temp_dir().join(name))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
