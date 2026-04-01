use std::sync::Arc;
use std::time::Instant;

use rand::{distributions::Alphanumeric, Rng};
use tokio::sync::Mutex;

use crate::config::RuntimeConfig;
use crate::error::KeystoneError;
use crate::http_server::bind_localhost;
use crate::pairing::PairingStore;
use crate::provider::ProviderRegistry;
use crate::session::SessionStore;
use crate::state_store::StateStore;
use crate::vault::{InMemorySecretStore, KeyringSecretStore, Vault};

#[derive(Clone)]
pub struct AppState {
    pub started_at: Instant,
    pub config: RuntimeConfig,
    pub state_store: StateStore,
    pub extension_id_seen: String,
    pub http_base_url: String,
    pub admin_token: String,
    pub http_client: reqwest::Client,
    pub providers: ProviderRegistry,
    pub pairing: Arc<Mutex<PairingStore>>,
    pub sessions: Arc<Mutex<SessionStore>>,
    pub vault: Arc<Mutex<Vault>>,
}

impl AppState {
    pub async fn new() -> Result<Self, KeystoneError> {
        let config = RuntimeConfig::load();
        let state_store = StateStore::new(config.flavor)?;
        let persisted_pairing =
            state_store.restore_pairing(config.flavor, &config.extension_id)?;
        let pairing = Arc::new(Mutex::new(PairingStore::from_record(persisted_pairing)));
        let sessions = Arc::new(Mutex::new(SessionStore::default()));
        let vault_backend = vault_backend(&config);
        let vault = Arc::new(Mutex::new(Vault::new(vault_backend)));

        let state = Self {
            started_at: Instant::now(),
            extension_id_seen: config.extension_id.clone(),
            config,
            state_store,
            http_base_url: String::new(),
            admin_token: random_token(),
            http_client: reqwest::Client::new(),
            providers: ProviderRegistry::default(),
            pairing,
            sessions,
            vault,
        };

        let http_addr = bind_localhost(state.clone()).await?;

        Ok(Self {
            http_base_url: format!("http://{http_addr}"),
            ..state
        })
    }
}

fn random_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn vault_backend(config: &RuntimeConfig) -> Box<dyn crate::vault::SecretStore> {
    if std::env::var("KEYSTONE_IN_MEMORY_VAULT").ok().as_deref() == Some("1") {
        return Box::<InMemorySecretStore>::default();
    }

    Box::new(KeyringSecretStore::new(config.flavor))
}
