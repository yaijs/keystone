use std::collections::HashMap;

use keyring::Entry;

use crate::config::HostFlavor;
use crate::protocol::{ProviderInfo, ProviderStatus};
use crate::provider::ProviderRegistry;

pub trait SecretStore: Send + Sync {
    fn set_secret(&mut self, provider: &str, secret: String) -> bool;
    fn get_secret(&self, provider: &str) -> Option<String>;
    fn delete_secret(&mut self, provider: &str) -> bool;
}

#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore for InMemorySecretStore {
    fn set_secret(&mut self, provider: &str, secret: String) -> bool {
        self.secrets.insert(provider.to_string(), secret);
        true
    }

    fn get_secret(&self, provider: &str) -> Option<String> {
        self.secrets.get(provider).cloned()
    }

    fn delete_secret(&mut self, provider: &str) -> bool {
        self.secrets.remove(provider).is_some()
    }
}

pub struct KeyringSecretStore {
    service_name: String,
}

impl KeyringSecretStore {
    pub fn new(flavor: HostFlavor) -> Self {
        Self {
            service_name: flavor.keyring_service_name().to_string(),
        }
    }

    fn entry(&self, provider: &str) -> Option<Entry> {
        Entry::new(&self.service_name, provider).ok()
    }
}

impl SecretStore for KeyringSecretStore {
    fn set_secret(&mut self, provider: &str, secret: String) -> bool {
        self.entry(provider)
            .and_then(|entry| entry.set_password(&secret).ok())
            .is_some()
    }

    fn get_secret(&self, provider: &str) -> Option<String> {
        self.entry(provider)
            .and_then(|entry| entry.get_password().ok())
    }

    fn delete_secret(&mut self, provider: &str) -> bool {
        self.entry(provider)
            .and_then(|entry| entry.delete_credential().ok())
            .is_some()
    }
}

#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub id: &'static str,
    pub display_name: &'static str,
    pub configured: bool,
}

pub struct Vault {
    backend: Box<dyn SecretStore>,
    providers: ProviderRegistry,
}

impl Vault {
    pub fn new(backend: Box<dyn SecretStore>) -> Self {
        Self {
            backend,
            providers: ProviderRegistry::default(),
        }
    }

    pub fn providers(&self) -> Vec<ProviderEntry> {
        self.providers
            .all()
            .iter()
            .map(|provider| ProviderEntry {
                id: provider.id,
                display_name: provider.display_name,
                configured: self.get_secret(provider.id).is_some(),
            })
            .collect()
    }

    pub fn list_provider_info(&self) -> Vec<ProviderInfo> {
        self.providers()
            .into_iter()
            .map(|provider| ProviderInfo {
                id: provider.id.to_string(),
                display_name: provider.display_name.to_string(),
                configured: provider.configured,
            })
            .collect()
    }

    pub fn list_provider_status(&self) -> Vec<ProviderStatus> {
        self.providers()
            .into_iter()
            .map(|provider| ProviderStatus {
                id: provider.id.to_string(),
                configured: provider.configured,
            })
            .collect()
    }

    pub fn is_provider_known(&self, provider: &str) -> bool {
        self.providers.get(provider).is_some()
    }

    pub fn is_configured(&self, provider: &str) -> bool {
        self.get_secret(provider).is_some()
    }

    pub fn get_secret(&self, provider: &str) -> Option<String> {
        self.backend.get_secret(provider)
    }

    pub fn set_secret(&mut self, provider: &str, secret: &str) -> bool {
        if !self.is_provider_known(provider) {
            return false;
        }
        self.backend.set_secret(provider, secret.to_string())
    }

    pub fn delete_secret(&mut self, provider: &str) -> bool {
        if !self.is_provider_known(provider) {
            return false;
        }
        self.backend.delete_secret(provider)
    }
}
