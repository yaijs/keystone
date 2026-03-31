use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub token: String,
    pub extension_id: String,
    pub provider_id: String,
    pub operation: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    sessions_by_scope: HashMap<(String, String), SessionRecord>,
    token_index: HashMap<String, (String, String)>,
}

impl SessionStore {
    pub fn create_session(
        &mut self,
        extension_id: String,
        provider_id: String,
        operation: String,
    ) -> SessionRecord {
        let record = SessionRecord {
            session_id: format!("sess_{}", Uuid::new_v4().simple()),
            token: random_token(),
            extension_id: extension_id.clone(),
            provider_id: provider_id.clone(),
            operation,
            expires_at: Utc::now() + Duration::minutes(5),
        };
        let key = (extension_id, provider_id);
        if let Some(previous) = self.sessions_by_scope.insert(key.clone(), record.clone()) {
            self.token_index.remove(&previous.token);
        }
        self.token_index.insert(record.token.clone(), key);
        record
    }

    pub fn count(&self) -> usize {
        self.sessions_by_scope.len()
    }

    pub fn validate_token(&self, token: &str, operation: &str) -> Option<SessionRecord> {
        let key = self.token_index.get(token)?;
        let record = self.sessions_by_scope.get(key)?;
        if record.expires_at <= Utc::now() || record.operation != operation {
            return None;
        }
        Some(record.clone())
    }

    pub fn validate_token_any(&self, token: &str) -> Option<SessionRecord> {
        let key = self.token_index.get(token)?;
        let record = self.sessions_by_scope.get(key)?;
        if record.expires_at <= Utc::now() {
            return None;
        }
        Some(record.clone())
    }
}

fn random_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}
