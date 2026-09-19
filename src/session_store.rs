use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::VortexAtomsError;
use crate::llm_prompt::{ChatMessage, MessageRole};
use crate::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: String,
    pub messages: Vec<SessionMessage>,
    pub metadata: SessionMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    pub model_arch: Option<String>,
    pub total_tokens: u64,
    pub temperature: Option<f64>,
}

pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        Self { base_dir }
    }

    pub fn default_dir() -> PathBuf {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(appdata)
                .join("vortex_atoms_ai")
                .join("sessions")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".vortex_atoms_ai")
                .join("sessions")
        } else {
            PathBuf::from(".").join("sessions")
        }
    }

    pub fn save(&self, session: &Session) -> Result<()> {
        // Session ids become filenames: keep them tightly scoped so a future
        // HTTP wiring cannot traverse directories (fails closed on `..`).
        let safe_id = crate::security::sanitize_session_id(&session.id).map_err(|e| {
            VortexAtomsError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })?;
        std::fs::create_dir_all(&self.base_dir).map_err(VortexAtomsError::Io)?;

        let path = self.base_dir.join(format!("{safe_id}.json"));
        let json = serde_json::to_string_pretty(session)
            .map_err(|e| VortexAtomsError::Gguf(e.to_string()))?;

        // Chat history is sensitive: encrypt at rest (Windows DPAPI,
        // user-scoped) and lock the file to the current user.
        let bytes = crate::security::protect_data(json.as_bytes())?;
        std::fs::write(&path, bytes).map_err(VortexAtomsError::Io)?;
        let _ = crate::security::restrict_to_current_user(&path);

        Ok(())
    }

    pub fn load(&self, session_id: &str) -> Result<Session> {
        let safe_id = crate::security::sanitize_session_id(session_id).map_err(|e| {
            VortexAtomsError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })?;
        let path = self.base_dir.join(format!("{safe_id}.json"));
        let bytes = std::fs::read(&path).map_err(VortexAtomsError::Io)?;
        // DPAPI blob, marked plaintext, or legacy raw JSON — all accepted.
        let plain = crate::security::unprotect_data(&bytes)?;
        let json = String::from_utf8(plain).map_err(|e| VortexAtomsError::Gguf(e.to_string()))?;

        let session: Session =
            serde_json::from_str(&json).map_err(|e| VortexAtomsError::Gguf(e.to_string()))?;

        Ok(session)
    }

    /// Delete session files older than `days` (by file mtime). Returns the
    /// number removed. `days == 0` disables purging.
    pub fn purge_older_than(dir: &std::path::Path, days: u64) -> usize {
        if days == 0 {
            return 0;
        }
        let cutoff = std::time::Duration::from_secs(days.saturating_mul(24 * 3600));
        let now = std::time::SystemTime::now();
        let mut removed = 0usize;
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| now.duration_since(t).ok())
                .map(|age| age > cutoff)
                .unwrap_or(false);
            if old && std::fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        removed
    }

    pub fn list_sessions(&self) -> Vec<String> {
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        sessions.push(name.strip_suffix(".json").unwrap_or(name).to_string());
                    }
                }
            }
        }

        sessions.sort();
        sessions
    }

    pub fn delete(&self, session_id: &str) -> Result<()> {
        let safe_id = crate::security::sanitize_session_id(session_id).map_err(|e| {
            VortexAtomsError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })?;
        let path = self.base_dir.join(format!("{safe_id}.json"));
        if path.exists() {
            std::fs::remove_file(&path).map_err(VortexAtomsError::Io)?;
        }
        Ok(())
    }
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            created_at: chrono_now(),
            messages: Vec::new(),
            metadata: SessionMetadata::default(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono_now(),
        });
    }

    pub fn to_chat_messages(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .filter_map(|m| {
                let role = match m.role.as_str() {
                    "system" => MessageRole::System,
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => return None,
                };
                Some(ChatMessage {
                    role,
                    content: m.content.clone(),
                })
            })
            .collect()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}s", d.as_secs()))
        .unwrap_or_default()
}
