use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostFlavor {
    Dev,
    Beta,
    Prod,
}

impl HostFlavor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Beta => "beta",
            Self::Prod => "prod",
        }
    }

    pub fn host_id(self) -> &'static str {
        match self {
            Self::Dev => "com.ytxt.keystone.dev",
            Self::Beta => "com.ytxt.keystone.beta",
            Self::Prod => "com.ytxt.keystone",
        }
    }

    pub fn keyring_service_name(self) -> &'static str {
        self.host_id()
    }
}

impl Default for HostFlavor {
    fn default() -> Self {
        Self::Prod
    }
}

impl FromStr for HostFlavor {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dev" => Ok(Self::Dev),
            "beta" => Ok(Self::Beta),
            "prod" => Ok(Self::Prod),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub flavor: HostFlavor,
    pub extension_origin: Option<String>,
    pub extension_id: String,
}

impl RuntimeConfig {
    pub fn load() -> Self {
        let flavor = std::env::var("KEYSTONE_FLAVOR")
            .ok()
            .and_then(|value| HostFlavor::from_str(&value).ok())
            .unwrap_or_default();

        let extension_origin = detect_extension_origin();
        let extension_id = std::env::var("KEYSTONE_EXTENSION_ID_OVERRIDE")
            .ok()
            .or_else(|| extension_origin.as_deref().and_then(parse_extension_id))
            .unwrap_or_else(|| "dev-extension-id".to_string());

        Self {
            flavor,
            extension_origin,
            extension_id,
        }
    }
}

fn detect_extension_origin() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|arg| arg.starts_with("chrome-extension://"))
}

fn parse_extension_id(origin: &str) -> Option<String> {
    origin
        .strip_prefix("chrome-extension://")
        .map(|value| value.trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}
