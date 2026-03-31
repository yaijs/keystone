use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::HostFlavor;
use crate::error::KeystoneError;
use crate::pairing::TrustRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedTrustRecord {
    pub host_flavor: String,
    pub extension_id: String,
    pub extension_name: String,
    pub allowed_providers: Vec<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub trust_record: Option<PersistedTrustRecord>,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(flavor: HostFlavor) -> Result<Self, KeystoneError> {
        if let Ok(override_dir) = std::env::var("KEYSTONE_STATE_DIR_OVERRIDE") {
            let path = PathBuf::from(override_dir).join(flavor.as_str()).join("state.json");
            return Ok(Self { path });
        }

        let data_dir = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .ok_or_else(|| KeystoneError::Internal("unable to resolve local data directory".to_string()))?;

        let path = data_dir
            .join("keystone")
            .join(flavor.as_str())
            .join("state.json");

        Ok(Self { path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn load(&self) -> Result<PersistedState, KeystoneError> {
        if !self.path.exists() {
            return Ok(PersistedState::default());
        }

        let raw = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save_pairing(&self, record: &TrustRecord) -> Result<(), KeystoneError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| KeystoneError::Internal("state path has no parent".to_string()))?;
        fs::create_dir_all(parent)?;

        let existing = self.load().unwrap_or_default();
        let now = Utc::now();

        let first_seen_at = existing
            .trust_record
            .as_ref()
            .filter(|stored| {
                stored.host_flavor == record.host_flavor.as_str()
                    && stored.extension_id == record.extension_id
            })
            .map(|stored| stored.first_seen_at)
            .unwrap_or(now);

        let persisted = PersistedState {
            trust_record: Some(PersistedTrustRecord {
                host_flavor: record.host_flavor.as_str().to_string(),
                extension_id: record.extension_id.clone(),
                extension_name: record.extension_name.clone(),
                allowed_providers: record.allowed_providers.clone(),
                first_seen_at,
                last_used_at: now,
            }),
        };

        let raw = serde_json::to_string_pretty(&persisted)?;
        fs::write(&self.path, raw)?;
        Ok(())
    }

    pub fn restore_pairing(
        &self,
        expected_flavor: HostFlavor,
        expected_extension_id: &str,
    ) -> Result<Option<TrustRecord>, KeystoneError> {
        let state = self.load()?;
        let Some(record) = state.trust_record else {
            return Ok(None);
        };

        if record.host_flavor != expected_flavor.as_str() || record.extension_id != expected_extension_id
        {
            return Ok(None);
        }

        Ok(Some(TrustRecord {
            host_flavor: expected_flavor,
            extension_id: record.extension_id,
            extension_name: record.extension_name,
            allowed_providers: record.allowed_providers,
        }))
    }
}
