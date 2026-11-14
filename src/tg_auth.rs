//! Telegram login for the in-app cache: `send_code` -> `verify_code` ->
//! optional 2FA password, with the MTProto session persisted in `filesDir`.
//!
//! Credentials (developer `api_id` / `api_hash`) are never bundled. They are
//! read from `TG_API_ID` / `TG_API_HASH`, or from `filesDir/tg_api.json`
//! (`{"api_id": 123, "api_hash": "..."}`).
//!
//! The signed-in session lives in `filesDir/aysha_tg.session` (JSON, see
//! `crate::tg_session`) so a restart does not require a new login code.

use grammers_client::{
    client::{Client, LoginToken, PasswordToken},
    SignInError, SenderPool,
};
use crate::tg_session::FileSession;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

pub const SESSION_FILE: &str = "aysha_tg.session";
pub const CREDENTIALS_FILE: &str = "tg_api.json";

#[derive(Debug, Clone)]
struct Credentials {
    api_id: i32,
    api_hash: String,
}

#[derive(Default)]
struct AuthState {
    client: Option<Arc<Client>>,
    api_hash: String,
    login_token: Option<LoginToken>,
    password_token: Option<PasswordToken>,
    authorized: Option<bool>,
}

/// Result of a login step, rendered as JSON for the HTTP API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TgOutcome {
    CodeSent,
    Authorized,
    PasswordRequired { hint: Option<String> },
    InvalidCode,
    InvalidPassword { hint: Option<String> },
    SignUpRequired,
    NotStarted,
    MissingCredentials,
    Error(String),
}

impl TgOutcome {
    pub fn to_json(&self) -> Value {
        match self {
            TgOutcome::CodeSent => json!({ "status": "code_sent" }),
            TgOutcome::Authorized => json!({ "status": "authorized", "authorized": true }),
            TgOutcome::PasswordRequired { hint } => json!({
                "status": "password_required",
                "hint": hint,
            }),
            TgOutcome::InvalidCode => json!({ "status": "invalid_code" }),
            TgOutcome::InvalidPassword { hint } => json!({
                "status": "invalid_password",
                "hint": hint,
            }),
            TgOutcome::SignUpRequired => json!({
                "status": "sign_up_required",
                "error": "create the account in an official Telegram app first",
            }),
            TgOutcome::NotStarted => json!({
                "status": "not_started",
                "error": "call /api/tg/send_code first",
            }),
            TgOutcome::MissingCredentials => json!({
                "status": "missing_credentials",
                "error": "set TG_API_ID/TG_API_HASH or filesDir/tg_api.json",
            }),
            TgOutcome::Error(message) => json!({ "status": "error", "error": message }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TgStatus {
    pub authorized: bool,
    pub connected: bool,
    pub needs_password: bool,
    pub pending_code: bool,
    pub configured: bool,
}

/// Login state machine shared by the `/api/tg/*` handlers.
pub struct TelegramAuth {
    files_dir: PathBuf,
    credentials: std::sync::RwLock<Option<Credentials>>,
    state: Mutex<AuthState>,
}

impl TelegramAuth {
    pub fn new(files_dir: impl AsRef<Path>) -> Self {
        let files_dir = files_dir.as_ref().to_path_buf();
        Self {
            credentials: std::sync::RwLock::new(load_credentials(&files_dir)),
            files_dir,
            state: Mutex::new(AuthState::default()),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.credentials.read().unwrap().is_some()
    }

    /// Configured only from the on-device setup screen; never returned by status.
    pub async fn configure(&self, api_id:i32, api_hash:&str)->Result<(), String> {
        if api_hash.trim().len()!=32 || !api_hash.trim().bytes().all(|b|b.is_ascii_hexdigit()) {return Err("API hash must be 32 hexadecimal characters".into());}
        let credentials=parse_credentials_file(&json!({"api_id":api_id,"api_hash":api_hash}).to_string())
            .ok_or("Enter a positive API ID and API hash")?;
        let state=self.state.lock().await;
        if state.client.is_some() {return Err("Disconnect Telegram before changing API credentials".into());}
        let bytes=serde_json::to_vec(&json!({"api_id":credentials.api_id,"api_hash":credentials.api_hash})).map_err(|_|"Could not save credentials")?;
        crate::tg_session::private_write(&self.files_dir.join(CREDENTIALS_FILE),&bytes).map_err(|_|"Could not save credentials")?;
        *self.credentials.write().unwrap()=Some(credentials);
        Ok(())
    }
    pub async fn disconnect(&self)->Result<(),String> {
        let mut state=self.state.lock().await;
        if let Some(client)=&state.client {
            client.sign_out().await.map_err(|_|"Could not revoke Telegram session. Retry when connected.")?;
        }
        *state=AuthState::default();
        match std::fs::remove_file(self.session_path()) {Ok(())=>(),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>(),Err(_)=>return Err("Could not remove local session".into())}
        Ok(())
    }

    pub fn session_path(&self) -> PathBuf {
        self.files_dir.join(SESSION_FILE)
    }

    /// Returns a connected client, creating the sender pool on first use.
    async fn client(&self) -> Result<(Arc<Client>, String), String> {
        let mut state = self.state.lock().await;
        if let Some(client) = &state.client {
            return Ok((client.clone(), state.api_hash.clone()));
        }

        let credentials = self.credentials.read().unwrap().clone()
            .ok_or_else(|| "missing_credentials".to_string())?;

        let session = FileSession::open(self.session_path())
            .map_err(|error| format!("session_open_failed: {error}"))?;

        let pool = SenderPool::new(Arc::new(session), credentials.api_id);
        let client = Arc::new(Client::new(pool.handle));
        // The runner drives all MTProto I/O; it stops when the client quits.
        tokio::spawn(pool.runner.run());

        state.client = Some(client.clone());
        state.api_hash = credentials.api_hash.clone();
        Ok((client, credentials.api_hash.clone()))
    }

    /// Step 1: ask Telegram to send a login code to `phone`.
    pub async fn send_code(&self, phone: &str) -> TgOutcome {
        let phone = phone.trim();
        if phone.is_empty() {
            return TgOutcome::Error("phone_required".to_string());
        }

        let (client, api_hash) = match self.client().await {
            Ok(pair) => pair,
            Err(error) if error == "missing_credentials" => return TgOutcome::MissingCredentials,
            Err(error) => return TgOutcome::Error(error),
        };

        if let Ok(true) = client.is_authorized().await {
            self.state.lock().await.authorized = Some(true);
            return TgOutcome::Authorized;
        }

        match client.request_login_code(phone, &api_hash).await {
            Ok(token) => {
                let mut state = self.state.lock().await;
                state.login_token = Some(token);
                state.password_token = None;
                TgOutcome::CodeSent
            }
            Err(error) => TgOutcome::Error(format!("request_code_failed: {error}")),
        }
    }

    /// Step 2: complete the login with the code from Telegram.
    ///
    /// When 2FA is enabled this returns `PasswordRequired` (with the hint) and
    /// `verify_password` finishes the login.
    pub async fn verify_code(&self, code: &str) -> TgOutcome {
        let code = code.trim();
        if code.is_empty() {
            return TgOutcome::Error("code_required".to_string());
        }

        let token = {
            let mut state = self.state.lock().await;
            match state.login_token.take() {
                Some(token) => token,
                None => return TgOutcome::NotStarted,
            }
        };

        let client = match self.client().await {
            Ok((client, _)) => client,
            Err(error) if error == "missing_credentials" => return TgOutcome::MissingCredentials,
            Err(error) => {
                // Keep the token so a transient failure can be retried.
                self.state.lock().await.login_token = Some(token);
                return TgOutcome::Error(error);
            }
        };

        match client.sign_in(&token, code).await {
            Ok(_user) => {
                let mut state = self.state.lock().await;
                state.authorized = Some(true);
                state.password_token = None;
                TgOutcome::Authorized
            }
            Err(SignInError::PasswordRequired(password_token)) => {
                let hint = password_token.hint().map(str::to_string);
                let mut state = self.state.lock().await;
                state.password_token = Some(password_token);
                TgOutcome::PasswordRequired { hint }
            }
            Err(SignInError::InvalidCode) => {
                self.state.lock().await.login_token = Some(token);
                TgOutcome::InvalidCode
            }
            Err(SignInError::SignUpRequired) => TgOutcome::SignUpRequired,
            Err(SignInError::InvalidPassword(password_token)) => {
                let hint = password_token.hint().map(str::to_string);
                self.state.lock().await.password_token = Some(password_token);
                TgOutcome::InvalidPassword { hint }
            }
            Err(SignInError::Other(error)) => {
                self.state.lock().await.login_token = Some(token);
                TgOutcome::Error(format!("sign_in_failed: {error}"))
            }
        }
    }

    /// Step 2b (optional): finish a 2FA login with the account password.
    pub async fn verify_password(&self, password: &str) -> TgOutcome {
        if password.is_empty() {
            return TgOutcome::Error("password_required".to_string());
        }

        let token = {
            let mut state = self.state.lock().await;
            match state.password_token.take() {
                Some(token) => token,
                None => return TgOutcome::NotStarted,
            }
        };

        let client = match self.client().await {
            Ok((client, _)) => client,
            Err(error) if error == "missing_credentials" => return TgOutcome::MissingCredentials,
            Err(error) => {
                self.state.lock().await.password_token = Some(token);
                return TgOutcome::Error(error);
            }
        };

        match client.check_password(token, password).await {
            Ok(_user) => {
                let mut state = self.state.lock().await;
                state.authorized = Some(true);
                state.login_token = None;
                TgOutcome::Authorized
            }
            Err(SignInError::InvalidPassword(password_token)) => {
                let hint = password_token.hint().map(str::to_string);
                self.state.lock().await.password_token = Some(password_token);
                TgOutcome::InvalidPassword { hint }
            }
            Err(SignInError::PasswordRequired(password_token)) => {
                let hint = password_token.hint().map(str::to_string);
                self.state.lock().await.password_token = Some(password_token);
                TgOutcome::PasswordRequired { hint }
            }
            Err(error) => TgOutcome::Error(format!("password_failed: {error}")),
        }
    }

    /// Current login state. Connects once (lazily) to learn whether the stored
    /// session is still authorized.
    pub async fn status(&self) -> TgStatus {
        let cached = {
            let state = self.state.lock().await;
            (
                state.authorized,
                state.password_token.is_some(),
                state.login_token.is_some(),
            )
        };
        let (cached_authorized, needs_password, pending_code) = cached;

        let authorized = match cached_authorized {
            Some(value) => value,
            None if self.is_configured() => match self.client().await {
                Ok((client, _)) => match client.is_authorized().await {
                    Ok(value) => {
                        self.state.lock().await.authorized = Some(value);
                        value
                    }
                    Err(_) => false,
                },
                Err(_) => false,
            },
            None => false,
        };

        let connected = self.state.lock().await.client.is_some();
        TgStatus {
            authorized,
            connected,
            needs_password,
            pending_code,
            configured: self.is_configured(),
        }
    }

    /// Signed-in client for uploads/downloads, if the login already completed.
    pub async fn authorized_client(&self) -> Option<Arc<Client>> {
        let authorized = self.status().await.authorized;
        if !authorized {
            return None;
        }
        self.client().await.ok().map(|(client, _)| client)
    }
}

fn load_credentials(files_dir: &Path) -> Option<Credentials> {
    if let (Ok(id), Ok(hash)) = (
        std::env::var("TG_API_ID"),
        std::env::var("TG_API_HASH"),
    ) {
        if let Ok(api_id) = id.trim().parse::<i32>() {
            if !hash.trim().is_empty() {
                return Some(Credentials {
                    api_id,
                    api_hash: hash.trim().to_string(),
                });
            }
        }
    }

    let text = std::fs::read_to_string(files_dir.join(CREDENTIALS_FILE)).ok()?;
    parse_credentials_file(&text)
}

#[derive(Deserialize)]
struct CredentialsFile {
    api_id: i32,
    api_hash: String,
}

fn parse_credentials_file(text: &str) -> Option<Credentials> {
    let parsed: CredentialsFile = serde_json::from_str(text).ok()?;
    let api_hash = parsed.api_hash.trim();
    if parsed.api_id <= 0 || api_hash.is_empty() || api_hash.len() > 128 {
        return None;
    }
    Some(Credentials {
        api_id: parsed.api_id,
        api_hash: api_hash.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_app_setup_validates_and_saves_privately_without_network() {
        let root=std::env::temp_dir().join(format!("aysha-config-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&root).unwrap();
        let auth=TelegramAuth::new(&root);
        assert!(auth.configure(0,&"a".repeat(32)).await.is_err());
        assert!(auth.configure(12345,"bad hash").await.is_err());
        auth.configure(12345,&"a".repeat(32)).await.unwrap();
        assert!(auth.is_configured());
        assert!(!root.join(SESSION_FILE).exists(),"setup must not contact Telegram or create login state");
        #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;assert_eq!(std::fs::metadata(root.join(CREDENTIALS_FILE)).unwrap().permissions().mode()&0o777,0o600);}
        assert!(TelegramAuth::new(&root).is_configured());
        let _=std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_credentials_keep_the_manager_unconfigured() {
        let root = std::env::temp_dir().join(format!(
            "aysha-tg-auth-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let auth = TelegramAuth::new(&root);
        assert!(!auth.is_configured());
        assert!(auth.session_path().ends_with(SESSION_FILE));
        assert_eq!(
            TgOutcome::MissingCredentials.to_json()["status"],
            "missing_credentials"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn credentials_file_is_validated() {
        assert!(parse_credentials_file(r#"{"api_id":12345,"api_hash":"abc123"}"#).is_some());
        assert!(parse_credentials_file(r#"{"api_id":0,"api_hash":"abc"}"#).is_none());
        assert!(parse_credentials_file(r#"{"api_id":12345,"api_hash":""}"#).is_none());
        assert!(parse_credentials_file("not json").is_none());
    }

    #[tokio::test]
    async fn pending_login_steps_fail_loudly_instead_of_guessing() {
        let root = std::env::temp_dir().join(format!(
            "aysha-tg-auth-test-pending-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let auth = TelegramAuth::new(&root);

        // No request_login_code happened yet, so verify_code/password refuse.
        assert_eq!(
            auth.verify_password("secret").await.to_json()["status"],
            "not_started"
        );
        assert_eq!(
            auth.verify_code("12345").await.to_json()["status"],
            "not_started"
        );
        let status = auth.status().await;
        assert!(!status.authorized && !status.configured && !status.needs_password);
        let _ = std::fs::remove_dir_all(&root);
    }
}
