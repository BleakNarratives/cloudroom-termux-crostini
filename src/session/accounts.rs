//! Harness account logins: Codex, Claude Code, and Cursor.
use super::*;

impl Manager {
    pub async fn claude_auth_status(&self) -> runtime::auth::Status {
        self.claude_login.status(&self.config).await
    }

    pub async fn claude_account(
        &self,
        action: &str,
        id: String,
        code: Option<String>,
        state: Option<String>,
    ) -> Result<runtime::auth::Status> {
        if self.is_stopping() {
            return Err(Error::Conflict("service is stopping"));
        }
        Ok(self
            .claude_login
            .action(&self.config, action, id, code, state)
            .await)
    }

    pub fn start_auth_monitor(&self) {
        self.claude_auth.start(&self.config, &self.observability);
    }

    pub async fn codex_auth_status(self: &Arc<Self>) -> runtime::auth::Status {
        let status = self.codex_auth.status(&self.config).await;
        if self.codex_auth.take_switched().await {
            self.continue_limited();
        }
        status
    }

    /// Verify a login installed outside the core, then continue quota-stopped work.
    pub async fn codex_switched(self: &Arc<Self>) -> (runtime::auth::Status, Vec<String>) {
        let status = self.codex_auth.verify(&self.config).await;
        let continued = if status.state == "connected" {
            self.continue_limited()
        } else {
            Vec::new()
        };
        (status, continued)
    }

    /// Send "keep working" to Codex and Pi sessions stopped by a usage limit.
    /// The harness restarts first, so it loads the new login before the prompt runs.
    fn continue_limited(self: &Arc<Self>) -> Vec<String> {
        if self.is_stopping() || self.storage.blocks() {
            return Vec::new();
        }
        let mut local = self.local.lock().unwrap();
        let ids: Vec<_> = local
            .sessions
            .values()
            .filter(|s| {
                s.usage_limited
                    && matches!(s.harness, runtime::Kind::Codex | runtime::Kind::Pi)
                    && s.ready
                    && s.handle.is_some()
                    && s.state == "idle"
                    && s.current_request.is_none()
                    && s.queue.is_empty()
                    && !s.queue_paused
                    && !s.compacting
                    && s.close_request.is_none()
                    && s.rewind_request.is_none()
            })
            .map(|s| s.session_id.clone())
            .collect();
        let mut continued = Vec::new();
        for id in ids {
            let receipt = Receipt {
                request_id: format!("continue_{}", now_ms()),
                command: "prompt".into(),
                input: json!({"text":"keep working"}),
                state: "accepted".into(),
                model: None,
                provider: None,
                workspace: None,
                error: None,
            };
            let Ok(data) = serde_json::to_value(&receipt) else {
                continue;
            };
            if local.append(&id, "receipt", data, None).is_err() {
                continue;
            }
            if let Some(handle) = &local.sessions[&id].handle {
                handle.request_shutdown();
            }
            continued.push(id);
        }
        continued
    }

    fn check_codex_login(&self) -> Result<()> {
        if self.is_stopping() {
            return Err(Error::Conflict("service is stopping"));
        }
        if self.storage.blocks() {
            return Err(Error::Conflict("storage unsafe; new execution is blocked"));
        }
        if self.local.lock().unwrap().sessions.values().any(|s| {
            s.harness == runtime::Kind::Codex
                && (s.current_request.is_some()
                    || (!s.queue_paused && !s.queue.is_empty())
                    || matches!(s.state.as_str(), "starting" | "resuming" | "pending"))
        }) {
            return Err(Error::Conflict(
                "finish active Codex work before signing in",
            ));
        }
        Ok(())
    }

    pub async fn codex_login(&self, request: String) -> Result<runtime::auth::Status> {
        self.check_codex_login()?;
        Ok(self.codex_auth.login(&self.config, request).await)
    }

    pub async fn import_codex_login(
        self: &Arc<Self>,
        credentials: Value,
    ) -> Result<runtime::auth::Status> {
        self.check_codex_login()?;
        let status = self
            .codex_auth
            .import(&self.config, &self.storage, credentials)
            .await;
        if self.codex_auth.take_switched().await {
            self.continue_limited();
        }
        Ok(status)
    }

    pub async fn cancel_codex_login(&self, request: &str) -> runtime::auth::Status {
        self.codex_auth.cancel(request).await
    }

    pub async fn cursor_auth_status(&self) -> runtime::auth::Status {
        self.cursor_auth.status(&self.config, &self.storage).await
    }
    pub async fn cursor_account(
        &self,
        action: &str,
        request: String,
        key: Option<String>,
    ) -> Result<runtime::auth::Status> {
        if action == "cancel" {
            return Ok(self.cursor_auth.cancel(&request).await);
        }
        if self.is_stopping() {
            return Err(Error::Conflict("service is stopping"));
        }
        if self.storage.blocks() {
            return Err(Error::Conflict("storage unsafe; new execution is blocked"));
        }
        if self.local.lock().unwrap().sessions.values().any(|session| {
            session.harness == runtime::Kind::Cursor
                && (session.handle.is_some()
                    || session.current_request.is_some()
                    || (!session.queue_paused && !session.queue.is_empty())
                    || matches!(session.state.as_str(), "starting" | "resuming" | "pending"))
        }) {
            return Err(Error::Conflict(
                "close Cursor sessions before changing the cloud login",
            ));
        }
        Ok(if let Some(key) = key {
            self.cursor_auth
                .set_key(&self.config, &self.storage, key)
                .await
        } else {
            self.cursor_auth
                .login(&self.config, &self.storage, request)
                .await
        })
    }
}
