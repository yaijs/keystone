use serde::Serialize;

use crate::config::HostFlavor;

#[derive(Debug, Clone, Serialize)]
pub struct NativeHostManifest {
    pub name: String,
    pub description: String,
    pub path: String,
    #[serde(rename = "type")]
    pub host_type: &'static str,
    pub allowed_origins: Vec<String>,
}

impl NativeHostManifest {
    pub fn for_flavor(flavor: HostFlavor, binary_path: &str, extension_id: &str) -> Self {
        Self {
            name: flavor.host_id().to_string(),
            description: format!(
                "Keystone native host for Y/TXT {} builds",
                match flavor {
                    HostFlavor::Dev => "development",
                    HostFlavor::Beta => "beta",
                    HostFlavor::Prod => "production",
                }
            ),
            path: binary_path.to_string(),
            host_type: "stdio",
            allowed_origins: vec![format!("chrome-extension://{extension_id}/")],
        }
    }
}
