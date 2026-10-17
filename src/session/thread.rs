//! Cloud agents rename or archive their own thread, change its effort, and start, stop and archive child threads in their own sandbox. Each request
//! is a session record, so the app applies it whenever it next reads the thread, including after it was offline.
//! A child gets a sandbox of its own by default (ADR 0229). The app starts and reaches it, so Core records the
//! request and waits briefly for the app's answer; without one, the child starts in this sandbox instead.
use super::Manager;
use crate::preview::Peer;
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{self, Read},
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

const USAGE: &str = "room-cli thread update --self --title TITLE
room-cli thread update --self --reasoning-level LEVEL
room-cli thread archive --self
room-cli thread stop --self
room-cli thread spawn --provider codex|claude-code|pi|opencode|hermes|devin [--model MODEL] [--reasoning-level LEVEL] [--title TITLE] [--shared-sandbox] --prompt TEXT|--prompt-file PATH
room-cli thread list [--include-archived]
room-cli thread output CHILD_ID [--all]
room-cli thread tell CHILD_ID TEXT
room-cli thread stop CHILD_ID
room-cli thread archive CHILD_ID";
const SPAWN_NOTE: &str = "Started in this sandbox and folder. Keep working: Cloudroom sends you a message each time it finishes a turn.";
const OWN_NOTE: &str = "Started in its own sandbox with a copy of your code (commits and uncommitted changes), on its own branch. It doesn't see your later edits; share work through Git. Use this ID with tell, stop and archive. Keep working: Cloudroom sends you a message each time it finishes a turn.";
const FALLBACK_NOTE: &str = " The user's Cloudroom app didn't start it in its own sandbox (it's probably closed), so it runs here.";
const OWN_COMMAND_NOTE: &str =
    "Passed to the user's Cloudroom app, which applies it while it is open.";
/// How long a spawn waits for the app to start the child in its own sandbox, and how long after a miss spawns
/// share this sandbox right away, so a closed app costs one wait, not one per child.
const APP_WAIT: Duration = Duration::from_secs(60);
const APP_MISSED_FOR: Duration = Duration::from_secs(300);
/// How long a child request's outcome is remembered, so a retried spawn or a repeated app answer gets the same one.
const OUTCOME_KEPT: Duration = Duration::from_secs(3600);
/// Each child request's outcome, by parent session and request ID. It is decided once, under this lock, so a
/// child never runs both in its own sandbox and here.
static CHILDREN: LazyLock<Mutex<HashMap<String, Outcome>>> = LazyLock::new(Mutex::default);
static APP_MISSED: Mutex<Option<Instant>> = Mutex::new(None);

enum Outcome {
    /// Waiting for the app: its thread ID, or why it could not start the child.
    Waiting(oneshot::Sender<Result<String, String>>),
    /// The app started the child in its own sandbox.
    Started(String, Instant),
    /// The app did not answer in time (or the agent gave up): the child runs here, and a late start is cancelled.
    Here(Instant),
}
pub(crate) type Failure = (StatusCode, Json<Value>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rename {
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnBody {
    request_id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    reasoning: Option<String>,
    title: Option<String>,
    prompt: String,
    /// Run the child in this sandbox and folder instead of a sandbox of its own (ADR 0229).
    #[serde(default)]
    shared: bool,
}

/// The app's answer to a child request: the child thread it started, or why it could not.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildAnswer {
    thread: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effort {
    reasoning: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tell {
    text: String,
}

fn valid(title: &str) -> bool {
    !title.is_empty() && title.chars().count() <= 200 && !title.chars().any(char::is_control)
}
/// The app answers child requests here.
pub fn routes() -> Router<Arc<Manager>> {
    Router::new().route("/v1/sessions/{id}/children/{request}", post(answer_child))
}

/// Served on the agent-only socket that `room-cli mac` also uses.
pub(crate) fn agent_routes() -> Router<Arc<Manager>> {
    Router::new()
        .route("/title", post(rename))
        .route("/reasoning", post(set_reasoning))
        .route("/archive", post(archive))
        .route("/children", get(list).post(spawn))
        .route("/children/{id}", get(output))
        .route("/children/{id}/messages", post(tell))
        .route("/children/{id}/stop", post(stop_child))
        .route("/children/{id}/archive", post(archive_child))
}

/// The session whose harness process sent this `cloudroom <command>` request.
pub(crate) fn caller(m: &Manager, peer: Peer, command: &str) -> Result<String, Failure> {
    if peer.0.is_none() || m.previews.agent().map(|(_, uid)| uid) != peer.0 {
        return Err(fail(
            StatusCode::FORBIDDEN,
            &format!("Only the VM agent account may run cloudroom {command}"),
        ));
    }
    peer.1
        .and_then(|pid| m.harness_session(&crate::secrets::ancestors(pid)))
        .ok_or(fail(
            StatusCode::CONFLICT,
            &format!("Run cloudroom {command} from a Cloudroom cloud thread"),
        ))
}

pub(crate) fn fail(status: StatusCode, error: &str) -> Failure {
    (status, Json(json!({"error":error})))
}

fn record(m: &Manager, session: &str, kind: &str, data: Value) -> Result<Json<Value>, Failure> {
    m.note(session, kind, data.clone()).map_err(|error| {
        fail(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("Could not record the {kind}: {error}"),
        )
    })?;
    Ok(Json(data))
}

async fn rename(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Json(input): Json<Rename>,
) -> Result<Json<Value>, Failure> {
    let session = caller(&m, peer, "thread")?;
    let title = input.title.trim();
    if !valid(title) {
        return Err(fail(
            StatusCode::CONFLICT,
            "Use a title of 1-200 characters",
        ));
    }
    record(&m, &session, "title", json!({"title":title}))
}

/// The new effort applies from this session's next turn; the app shows it as the thread's effort.
async fn set_reasoning(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Json(input): Json<Effort>,
) -> Result<Json<Value>, Failure> {
    let session = caller(&m, peer, "thread")?;
    m.check_prompt_reasoning(&session, Some(&input.reasoning))
        .await
        .map_err(session_failure)?;
    record(
        &m,
        &session,
        "reasoning",
        json!({"reasoning":input.reasoning}),
    )
}

/// The app archives the thread and its children, which also stops this session.
async fn archive(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
) -> Result<Json<Value>, Failure> {
    let session = caller(&m, peer, "thread")?;
    archive_under(&m, &session)?;
    record(&m, &session, "archive", json!({"archived":true}))
}

/// Stops and archives every thread under `id` now: the app archives them too, but never stops them,
/// and it may be offline. An archived thread is stopped again, in case it still runs.
fn archive_under(m: &Arc<Manager>, id: &str) -> Result<(), Failure> {
    for (child, archived) in m.descendants(id) {
        let request =
            random_key().map_err(|e| fail(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()))?;
        let _ = m.command(&child, request, "stop", json!({}));
        if !archived {
            let _ = record(m, &child, "archive", json!({"id":child,"archived":true}))?;
        }
    }
    Ok(())
}

fn session_failure(error: super::Error) -> Failure {
    let status = match error {
        super::Error::NotFound => StatusCode::NOT_FOUND,
        super::Error::Conflict(_) => StatusCode::CONFLICT,
        super::Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
    };
    fail(status, &error.message())
}

fn random_key() -> io::Result<String> {
    let mut bytes = [0u8; 12];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The calling thread starts a child in its own sandbox; Core tells the parent when each child turn ends.
async fn spawn(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Json(input): Json<SpawnBody>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread spawn")?;
    let bad = |message: &str| fail(StatusCode::CONFLICT, message);
    if input.prompt.trim().is_empty() || input.prompt.len() > 32768 {
        return Err(bad("Use a prompt of 1-32768 bytes"));
    }
    if input.title.as_deref().is_some_and(|t| !valid(t.trim())) {
        return Err(bad("Use a title of 1-200 characters"));
    }
    if input
        .model
        .as_ref()
        .is_some_and(|m| m.is_empty() || m.len() > 256 || m.chars().any(char::is_control))
    {
        return Err(bad("invalid model"));
    }
    if input
        .reasoning
        .as_ref()
        .is_some_and(|r| r.is_empty() || r.len() > 64 || !r.bytes().all(|b| b.is_ascii_lowercase()))
    {
        return Err(bad("invalid reasoning level"));
    }
    let harness = input
        .provider
        .as_deref()
        .map(|p| {
            serde_json::from_value(json!(p)).map_err(|_| {
                bad("Use --provider codex, claude-code, pi, opencode, hermes or devin")
            })
        })
        .transpose()?;
    // Only a Pi parent passes its own model on; Cloud's default Pi model may not work with the user's logins.
    let pi_parent = m
        .local
        .lock()
        .unwrap()
        .sessions
        .get(&parent)
        .map(|p| p.harness)
        == Some(crate::runtime::Kind::Pi);
    if harness == Some(crate::runtime::Kind::Pi)
        && !pi_parent
        && !input
            .model
            .as_deref()
            .and_then(|m| m.split_once('/'))
            .is_some_and(|(p, n)| !p.is_empty() && !n.is_empty())
    {
        return Err(bad(
            "Select a Pi provider and model: add --model provider/model, for example anthropic/claude-haiku-5-5",
        ));
    }
    let key = match input.request_id.clone() {
        Some(key) => key,
        None => random_key().map_err(|e| fail(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()))?,
    };
    if key.is_empty()
        || key.len() > 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(bad(
            "request_id must be 1-64 ASCII letters, digits, underscores or hyphens",
        ));
    }
    let mut note = SPAWN_NOTE.to_owned();
    if !input.shared {
        match own_sandbox_child(&m, &parent, &key, &input).await? {
            Some(thread) => return Ok(Json(json!({"id":thread,"note":OWN_NOTE}))),
            None => note.push_str(FALLBACK_NOTE),
        }
    }
    let spawn = super::Spawn {
        harness,
        model: input.model,
        reasoning: input.reasoning,
        title: input.title.map(|t| t.trim().to_owned()),
        prompt: input.prompt,
    };
    let (id, receipt) = m
        .spawn_child(&parent, &key, spawn)
        .await
        .map_err(session_failure)?;
    Ok(Json(
        json!({"id":id,"harness":receipt.input["harness"],"model":receipt.model,"note":note}),
    ))
}

/// Asks the app to start the child in a sandbox of its own with a copy of this thread's code, and waits for
/// its answer. None means no answer: the app is closed or too old, so the child starts in this sandbox.
async fn own_sandbox_child(
    m: &Manager,
    parent: &str,
    key: &str,
    input: &SpawnBody,
) -> Result<Option<String>, Failure> {
    let id = format!("own_{key}");
    let slot = format!("{parent}/{id}");
    let (reply, answer) = oneshot::channel();
    {
        let mut children = CHILDREN.lock().unwrap();
        children.retain(|_, outcome| match outcome {
            Outcome::Started(_, at) | Outcome::Here(at) => at.elapsed() < OUTCOME_KEPT,
            Outcome::Waiting(_) => true,
        });
        match children.get(&slot) {
            Some(Outcome::Started(thread, _)) => return Ok(Some(thread.clone())),
            Some(Outcome::Here(_)) => return Ok(None),
            Some(Outcome::Waiting(_)) => {
                return Err(fail(
                    StatusCode::CONFLICT,
                    "A child with this request_id is already starting",
                ));
            }
            None => {}
        }
        if APP_MISSED
            .lock()
            .unwrap()
            .is_some_and(|at| at.elapsed() < APP_MISSED_FOR)
        {
            return Ok(None);
        }
        children.insert(slot.clone(), Outcome::Waiting(reply));
    }
    // If the agent stops waiting before the app answers, the child runs nowhere and a late start is cancelled.
    struct Pending(String);
    impl Drop for Pending {
        fn drop(&mut self) {
            let mut children = CHILDREN.lock().unwrap();
            if let Some(outcome @ Outcome::Waiting(_)) = children.get_mut(&self.0) {
                *outcome = Outcome::Here(Instant::now());
            }
        }
    }
    let _pending = Pending(slot.clone());
    let mut data = json!({"id":id,"prompt":input.prompt});
    for (name, value) in [
        ("provider", &input.provider),
        ("model", &input.model),
        ("reasoning", &input.reasoning),
        ("title", &input.title.as_ref().map(|t| t.trim().to_owned())),
    ] {
        if let Some(value) = value {
            data[name] = json!(value);
        }
    }
    let _ = record(m, parent, "child_request", data)?;
    let answer = tokio::time::timeout(APP_WAIT, answer).await;
    match answer {
        Ok(Ok(Ok(thread))) => {
            *APP_MISSED.lock().unwrap() = None;
            Ok(Some(thread))
        }
        Ok(Ok(Err(error))) => Err(fail(
            StatusCode::CONFLICT,
            &format!(
                "The child could not start in its own sandbox: {error}. Retry, or add --shared-sandbox to start it here."
            ),
        )),
        // No answer in time: decide under the lock, since the app may be answering right now.
        _ => {
            let mut children = CHILDREN.lock().unwrap();
            match children.get_mut(&slot) {
                Some(Outcome::Started(thread, _)) => Ok(Some(thread.clone())),
                Some(outcome) => {
                    *outcome = Outcome::Here(Instant::now());
                    *APP_MISSED.lock().unwrap() = Some(Instant::now());
                    Ok(None)
                }
                None => Ok(None),
            }
        }
    }
}

/// The app started the requested child, or says why it could not. A request that already ended the other way
/// (the child runs in the parent's sandbox) or is unknown is refused, so the app cancels its child.
async fn answer_child(
    Path((session, id)): Path<(String, String)>,
    Json(input): Json<ChildAnswer>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let accepted = Ok((StatusCode::ACCEPTED, Json(json!({"accepted":true}))));
    let slot = format!("{session}/{id}");
    let mut children = CHILDREN.lock().unwrap();
    match children.remove(&slot) {
        None => Err(fail(
            StatusCode::NOT_FOUND,
            "Unknown or expired child request",
        )),
        Some(Outcome::Waiting(reply)) => {
            let result = match input.thread {
                Some(thread) => {
                    children.insert(slot, Outcome::Started(thread.clone(), Instant::now()));
                    Ok(thread)
                }
                None => Err(input.error.unwrap_or_else(|| "unknown error".into())),
            };
            let _ = reply.send(result);
            accepted
        }
        Some(outcome) => {
            // The same answer again (say, after a lost response) is fine; anything else lost the race.
            let same = matches!((&outcome, &input.thread), (Outcome::Started(started, _), Some(thread)) if started == thread);
            children.insert(slot, outcome);
            if same {
                accepted
            } else {
                Err(fail(
                    StatusCode::CONFLICT,
                    "This child request already ended: the child runs in the parent's sandbox",
                ))
            }
        }
    }
}

/// `tell`, `stop` and `archive` for a child in its own sandbox, named by its thread ID: the app passes them on.
fn own_sandbox_command(
    m: &Manager,
    parent: &str,
    id: &str,
    action: &str,
    text: Option<&str>,
) -> Option<Result<Json<Value>, Failure>> {
    let own = id.strip_prefix("thr_").is_some_and(|key| {
        !key.is_empty()
            && key.len() <= 60
            && key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    });
    own.then(|| {
        let mut data = json!({"id":id,"action":action});
        if let Some(text) = text {
            data["text"] = json!(text);
        }
        record(m, parent, "child_command", data)
            .map(|_| Json(json!({"id":id,"state":"requested","note":OWN_COMMAND_NOTE})))
    })
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    include_archived: bool,
}

async fn list(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread list")?;
    Ok(Json(
        m.children(&parent, None, query.include_archived).await,
    ))
}

#[derive(Deserialize)]
struct OutputQuery {
    #[serde(default)]
    all: bool,
}

async fn output(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Path(id): Path<String>,
    Query(query): Query<OutputQuery>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread output")?;
    if id.starts_with("thr_") {
        // A child in its own sandbox: its saved chat, through the website, which checks this thread started it (ADR 0256).
        let Some(api) = &m.history.api else {
            return Ok(Json(
                json!({"id":id,"note":"This child runs in its own sandbox. Its replies arrive as messages when its turns end."}),
            ));
        };
        let all = if query.all { "1" } else { "0" };
        return api
            .get("chat", &[("thread", &id), ("all", all)])
            .await
            .map(Json)
            .map_err(|error| match error {
                super::history::ApiError::Status(status, message) => fail(
                    StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                    &message,
                ),
                error => fail(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &format!("Could not read the child's chat: {error}"),
                ),
            });
    }
    m.own_child(&parent, &id).map_err(session_failure)?;
    Ok(Json(
        m.children(&parent, Some(&id), true).await["children"][0].clone(),
    ))
}

async fn tell(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Path(id): Path<String>,
    Json(input): Json<Tell>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread tell")?;
    if input.text.trim().is_empty() || input.text.len() > 32768 {
        return Err(fail(StatusCode::CONFLICT, "Use a message of 1-32768 bytes"));
    }
    if let Some(answer) = own_sandbox_command(&m, &parent, &id, "tell", Some(&input.text)) {
        return answer;
    }
    m.own_child(&parent, &id).map_err(session_failure)?;
    let request =
        random_key().map_err(|e| fail(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()))?;
    match steer(&m, &id, &format!("{request}_steer"), &input.text)
        .await
        .as_deref()
    {
        Some("completed") => {
            return Ok(Json(
                json!({"id":id,"state":"steered","note":"Sent into the child's running turn. Cloudroom tells you when that turn finishes."}),
            ));
        }
        Some(state) => {
            return Ok(Json(
                json!({"id":id,"state":state,"note":"Could not confirm the child's running turn got it. Check thread output before sending it again."}),
            ));
        }
        None => {}
    }
    let receipt = m
        .command(&id, request, "prompt", json!({"text":input.text}))
        .map_err(session_failure)?;
    Ok(Json(
        json!({"id":id,"state":receipt.state,"note":"Queued. Cloudroom tells you when the child finishes it."}),
    ))
}

/// Sends a busy child the message in its running turn, as in Local threads, so it never waits
/// behind older messages. None means it was not delivered, so it can safely queue instead.
async fn steer(m: &Arc<Manager>, id: &str, request: &str, text: &str) -> Option<String> {
    let target = m
        .local
        .lock()
        .unwrap()
        .sessions
        .get(id)?
        .current_request
        .clone()?;
    let mut changed = m.subscribe();
    let input = json!({"text":text,"target_request_id":target});
    m.command(id, request.to_owned(), "steer", input).ok()?;
    let settled = async {
        loop {
            let state = m.receipt(id, request).map(|r| r.state).unwrap_or_default();
            if state != "accepted" || changed.changed().await.is_err() {
                return state;
            }
        }
    };
    let state = tokio::time::timeout(Duration::from_secs(10), settled)
        .await
        .unwrap_or_else(|_| "accepted".into());
    (state != "failed").then_some(state)
}

/// Stops the child's running turn and pauses its queue, like the app's stop button.
async fn stop_child(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Path(id): Path<String>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread stop")?;
    if let Some(answer) = own_sandbox_command(&m, &parent, &id, "stop", None) {
        return answer;
    }
    m.own_child(&parent, &id).map_err(session_failure)?;
    let request =
        random_key().map_err(|e| fail(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()))?;
    let receipt = m
        .command(&id, request, "stop", json!({}))
        .map_err(session_failure)?;
    Ok(Json(json!({"id":id,"state":receipt.state})))
}

/// Stops the child and every thread under it now; the app archives them whenever it next reads the thread.
async fn archive_child(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Path(id): Path<String>,
) -> Result<Json<Value>, Failure> {
    let parent = caller(&m, peer, "thread archive")?;
    if let Some(answer) = own_sandbox_command(&m, &parent, &id, "archive", None) {
        return answer;
    }
    m.own_child(&parent, &id).map_err(session_failure)?;
    let request =
        random_key().map_err(|e| fail(StatusCode::SERVICE_UNAVAILABLE, &e.to_string()))?;
    // A child that already ended has nothing to stop. Stop it first, so it starts no new children.
    let _ = m.command(&id, request, "stop", json!({}));
    archive_under(&m, &id)?;
    record(&m, &id, "archive", json!({"id":id,"archived":true}))
}

/// Flag value after `name`, for `room-cli thread spawn`.
fn flag<'a>(words: &[&'a str], name: &str) -> Option<&'a str> {
    words
        .iter()
        .position(|w| *w == name)
        .and_then(|i| words.get(i + 1).copied())
}

async fn spawn_cli(words: &[&str]) -> io::Result<i32> {
    // A child gets a sandbox of its own unless --shared-sandbox (ADR 0229); `--machine cloud` is a harmless Local habit.
    if flag(words, "--machine")
        .or(flag(words, "--host"))
        .is_some_and(|m| m != "cloud")
    {
        eprintln!(
            "Cloud threads start cloud children only. Omit --machine; add --shared-sandbox to share this sandbox."
        );
        return Ok(2);
    }
    let prompt = match (flag(words, "--prompt"), flag(words, "--prompt-file")) {
        (Some(text), None) => text.to_owned(),
        (None, Some(path)) => std::fs::read_to_string(path)?,
        _ => {
            println!("{USAGE}");
            return Ok(2);
        }
    };
    let mut body = json!({"prompt":prompt,"shared":words.contains(&"--shared-sandbox")});
    for (name, key) in [
        ("--provider", "provider"),
        ("--model", "model"),
        ("--reasoning-level", "reasoning"),
        ("--title", "title"),
        ("--request-id", "request_id"),
    ] {
        if let Some(value) = flag(words, name) {
            body[key] = json!(value);
        }
    }
    let answer = crate::mac::request(crate::mac::SOCKET, "POST", "/children", Some(body)).await?;
    println!("{answer}");
    Ok(0)
}

/// The same `room-cli thread ... --self` forms local threads use. `--json` is accepted and
/// ignored because the answer is always JSON.
pub async fn cli(args: &[String]) -> io::Result<i32> {
    let words: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|word| *word != "--json")
        .collect();
    let help = words.iter().any(|word| matches!(*word, "--help" | "-h"));
    let (path, body) = match words.as_slice() {
        _ if help => {
            println!("{USAGE}");
            return Ok(0);
        }
        ["spawn", rest @ ..] => return spawn_cli(rest).await,
        ["list", rest @ ..] => {
            let path = if rest.contains(&"--include-archived") {
                "/children?include_archived=true"
            } else {
                "/children"
            };
            let answer = crate::mac::request(crate::mac::SOCKET, "GET", path, None).await?;
            println!("{answer}");
            return Ok(0);
        }
        ["output", id] | ["output", id, "--all"] | ["output", "--all", id] => {
            let all = if words.contains(&"--all") {
                "?all=true"
            } else {
                ""
            };
            let answer = crate::mac::request(
                crate::mac::SOCKET,
                "GET",
                &format!("/children/{id}{all}"),
                None,
            )
            .await?;
            println!("{answer}");
            return Ok(0);
        }
        ["tell", id, text] | ["tell", id, "--prompt", text] => {
            let answer = crate::mac::request(
                crate::mac::SOCKET,
                "POST",
                &format!("/children/{id}/messages"),
                Some(json!({"text":text})),
            )
            .await?;
            println!("{answer}");
            return Ok(0);
        }
        ["update", "--self", "--title", title] | ["update", "--title", title, "--self"] => {
            ("/title", Some(json!({"title":title})))
        }
        ["update", "--self", "--reasoning-level", level]
        | ["update", "--reasoning-level", level, "--self"] => {
            ("/reasoning", Some(json!({"reasoning":level})))
        }
        ["archive", "--self"] => ("/archive", None),
        ["stop", "--self"] => {
            println!(
                "{}",
                json!({"stopped":true,"note":"Archiving a cloud thread stops it; nothing else to do."})
            );
            return Ok(0);
        }
        [action @ ("stop" | "archive"), id] if !id.starts_with('-') => {
            let answer = crate::mac::request(
                crate::mac::SOCKET,
                "POST",
                &format!("/children/{id}/{action}"),
                None,
            )
            .await?;
            println!("{answer}");
            return Ok(0);
        }
        _ => {
            println!(
                "{USAGE}\nRenames, archives or changes the effort of this cloud thread, or starts, checks, stops and archives its child threads."
            );
            return Ok(if words.is_empty() { 0 } else { 2 });
        }
    };
    let answer = crate::mac::request(crate::mac::SOCKET, "POST", path, body).await?;
    println!("{answer}");
    Ok(0)
}
