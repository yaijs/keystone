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
    fn sweep_expired(&mut self) {
        let now = Utc::now();
        let expired_tokens: Vec<String> = self
            .token_index
            .iter()
            .filter_map(|(token, key)| {
                let record = self.sessions_by_scope.get(key)?;
                (record.expires_at <= now).then(|| token.clone())
            })
            .collect();

        for token in expired_tokens {
            if let Some(key) = self.token_index.remove(&token) {
                self.sessions_by_scope.remove(&key);
            }
        }
    }

    pub fn create_session(
        &mut self,
        extension_id: String,
        provider_id: String,
        operation: String,
    ) -> SessionRecord {
        self.sweep_expired();
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

    pub fn validate_token(&mut self, token: &str, operation: &str) -> Option<SessionRecord> {
        self.sweep_expired();
        let key = self.token_index.get(token)?;
        let record = self.sessions_by_scope.get(key)?;
        if record.expires_at <= Utc::now() || record.operation != operation {
            return None;
        }
        Some(record.clone())
    }

    pub fn validate_token_any(&mut self, token: &str) -> Option<SessionRecord> {
        self.sweep_expired();
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
