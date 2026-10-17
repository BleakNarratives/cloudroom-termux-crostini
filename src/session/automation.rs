//! Cloud agents schedule work for their own thread (ADR 0212). `room-cli loop` is the same command: Automations are
//! called Loops in the app (ADR 0259). Core passes each command to the website (ADR 0234),
//! like feedback, so it works while the user's Mac is offline. The website's every-minute cron wakes the sandbox and
//! sends the prompt; it also computes the next run, so Core never parses cron.
use super::{
    Manager,
    thread::{Failure, caller, fail},
};
use crate::preview::Peer;
use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{io, sync::Arc};

const USAGE: &str = "room-cli automation create --name NAME --prompt TEXT|--prompt-file PATH (--cron 'MIN HOUR DAY MONTH WEEKDAY' [--timezone ZONE] | --at ISO_TIME | --in 30m) [--new-thread|--same-thread] [--script TEXT|--script-file PATH]
room-cli automation list
room-cli automation update ID [--name NAME] [--prompt TEXT] [--cron EXPR [--timezone ZONE] | --at ISO_TIME | --in 30m] [--script TEXT|--script-file PATH]
room-cli automation pause|resume|run|delete ID
A loop runs on a schedule even while the user's Mac is off (`room-cli loop` is the same command): every minute with a script, else at most every 15 minutes. Each --cron run starts a fresh Cloud thread in its own sandbox, from this thread's project and agent; --same-thread re-prompts this thread instead. A one-time --at/--in run re-prompts this thread; --new-thread gives it a fresh one. A fresh-thread automation saves this thread's settings, so it keeps running after this thread is archived. A script runs first (60 s), in the automation's own sandbox with fresh threads, else in this one: no output skips the run, and its output is added to the prompt. Cloudroom fills in the next run within a minute; check `list` for it or an error.";
const MAX_PROMPT: usize = 32_000;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cron: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_thread: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<String>,
}

/// Served on the agent-only socket that `room-cli mac` also uses.
pub(crate) fn agent_routes() -> Router<Arc<Manager>> {
    Router::new().route("/automation", post(automation))
}

async fn automation(
    State(m): State<Arc<Manager>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    Json(input): Json<Body>,
) -> Result<Json<Value>, Failure> {
    let session = caller(&m, peer, "automation")?;
    check(&input)?;
    let Some(api) = &m.history.api else {
        return Err(fail(
            StatusCode::CONFLICT,
            "Run room-cli automation from a Cloudroom cloud thread",
        ));
    };
    // The website finds the thread from this Core's own sign-in; the session is the agent asking.
    let mut body =
        serde_json::to_value(&input).map_err(|e| fail(StatusCode::BAD_REQUEST, &e.to_string()))?;
    body["session"] = json!(session);
    api.post("automation", &body)
        .await
        .map(Json)
        .map_err(|error| match error {
            super::history::ApiError::Status(status, message) => fail(
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
                &message,
            ),
            error => fail(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Could not save the automation: {error}"),
            ),
        })
}

/// Clear errors before the website, which checks the cron itself.
fn check(input: &Body) -> Result<(), Failure> {
    let bad = |message: &str| Err(fail(StatusCode::BAD_REQUEST, message));
    if let Some(id) = &input.id
        && (id.len() != 36 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
    {
        return bad("Invalid automation ID");
    }
    if input
        .name
        .as_ref()
        .is_some_and(|n| n.trim().is_empty() || n.chars().count() > 200)
    {
        return bad("Use a name of 1-200 characters");
    }
    if input
        .prompt
        .as_ref()
        .is_some_and(|p| p.trim().is_empty() || p.len() > MAX_PROMPT)
    {
        return bad("Use a prompt of 1-32000 bytes");
    }
    if input
        .script
        .as_ref()
        .is_some_and(|s| s.trim().is_empty() || s.len() > MAX_PROMPT)
    {
        return bad("Use a script of 1-32000 bytes");
    }
    if input
        .cron
        .as_ref()
        .is_some_and(|c| c.split_whitespace().count() != 5)
    {
        return bad("Use a 5-field cron: minute hour day month weekday");
    }
    let schedules = [
        input.cron.is_some(),
        input.at.is_some(),
        input.delay.is_some(),
    ]
    .iter()
    .filter(|s| **s)
    .count();
    if schedules > 1
        || (input.action == "create"
            && (schedules == 0 || input.name.is_none() || input.prompt.is_none()))
    {
        return bad("Give a name, a prompt, and one schedule: --cron, --at, or --in");
    }
    Ok(())
}

fn flag<'a>(words: &[&'a str], name: &str) -> Option<&'a str> {
    words
        .iter()
        .position(|w| *w == name)
        .and_then(|i| words.get(i + 1).copied())
}

/// `--in 30m` as `30 minutes`.
fn delay(value: &str) -> Option<String> {
    let split = value.find(|c: char| !c.is_ascii_digit())?;
    let (count, unit) = value.split_at(split);
    let unit = match unit {
        "s" => "seconds",
        "m" => "minutes",
        "h" => "hours",
        "d" => "days",
        _ => return None,
    };
    (count.parse::<u32>().ok()? > 0).then(|| format!("{count} {unit}"))
}

/// `room-cli automation ...` in Cloud threads; on the Mac, `room-cli automation create --cloud` does the same.
pub async fn cli(args: &[String]) -> io::Result<i32> {
    let words: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|w| *w != "--json")
        .collect();
    if words.is_empty() || words.iter().any(|w| matches!(*w, "--help" | "-h")) {
        println!("{USAGE}");
        return Ok(0);
    }
    let mut body = json!({"action": words[0]});
    match words.as_slice() {
        ["list"] => {}
        ["pause" | "resume" | "run" | "delete", id] => body["id"] = json!(id),
        ["create", ..] | ["update", ..] => {
            if words[0] == "update" {
                let Some(id) = words.get(1).filter(|w| !w.starts_with('-')) else {
                    println!("{USAGE}");
                    return Ok(2);
                };
                body["id"] = json!(id);
            }
            let prompt = match (flag(&words, "--prompt"), flag(&words, "--prompt-file")) {
                (Some(text), None) => Some(text.to_owned()),
                (None, Some(path)) => Some(std::fs::read_to_string(path)?),
                (None, None) => None,
                _ => {
                    println!("{USAGE}");
                    return Ok(2);
                }
            };
            if let Some(prompt) = prompt {
                body["prompt"] = json!(prompt);
            }
            let script = match (flag(&words, "--script"), flag(&words, "--script-file")) {
                (Some(text), None) => Some(text.to_owned()),
                (None, Some(path)) => Some(std::fs::read_to_string(path)?),
                (None, None) => None,
                _ => {
                    println!("{USAGE}");
                    return Ok(2);
                }
            };
            if let Some(script) = script {
                body["script"] = json!(script);
            }
            if words[0] == "create" && words.contains(&"--new-thread") {
                body["new_thread"] = json!(true);
            } else if words[0] == "create" && words.contains(&"--same-thread") {
                body["new_thread"] = json!(false);
            }
            for (name, key) in [
                ("--name", "name"),
                ("--cron", "cron"),
                ("--timezone", "timezone"),
                ("--at", "at"),
            ] {
                if let Some(value) = flag(&words, name) {
                    body[key] = json!(value);
                }
            }
            if let Some(value) = flag(&words, "--in") {
                let Some(interval) = delay(value) else {
                    eprintln!("Use --in with a number and s, m, h or d, like 30m.");
                    return Ok(2);
                };
                body["delay"] = json!(interval);
            }
        }
        _ => {
            println!("{USAGE}");
            return Ok(2);
        }
    }
    let answer = crate::mac::request(crate::mac::SOCKET, "POST", "/automation", Some(body)).await?;
    println!("{answer}");
    Ok(0)
}
