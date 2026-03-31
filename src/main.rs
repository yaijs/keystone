use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use keystone::app::AppState;
use keystone::config::{HostFlavor, RuntimeConfig};
use keystone::error::KeystoneError;
use keystone::manifest::NativeHostManifest;
use keystone::native_messaging::run_native_host;
use keystone::pairing::TrustRecord;
use keystone::state_store::StateStore;
use keystone::vault::{KeyringSecretStore, Vault};
use serde::Serialize;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), KeystoneError> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("serve") => return run_standalone().await,
        Some("status") => return run_status(),
        Some("detect") => return run_detect(),
        Some("install") => return run_install(&args[2..]),
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            return Ok(());
        }
        _ => {}
    }

    let state = AppState::new().await?;
    run_native_host(state).await
}

async fn run_standalone() -> Result<(), KeystoneError> {
    let state = AppState::new().await?;
    eprintln!("Keystone standalone mode");
    eprintln!("Flavor: {}", state.config.flavor.as_str());
    eprintln!("Admin UI: {}/admin", state.http_base_url);
    eprintln!("Tunnel base URL: {}", state.http_base_url);
    eprintln!("Press Ctrl+C to stop.");

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| KeystoneError::Internal(format!("failed to wait for ctrl+c: {err}")))?;

    Ok(())
}

fn run_status() -> Result<(), KeystoneError> {
    let json = env::args().skip(2).any(|arg| arg == "--json");
    let status = collect_status()?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&status)
                .map_err(|err| KeystoneError::Internal(format!("failed to serialize status json: {err}")))?,
        );
        return Ok(());
    }

    print_status_human(&status);
    Ok(())
}

fn collect_status() -> Result<StatusOutput, KeystoneError> {
    let config = RuntimeConfig::load();
    let state_store = StateStore::new(config.flavor)?;
    let persisted_pairing = state_store
        .restore_pairing(config.flavor, &config.extension_id)?
        .or_else(|| state_store.load().ok().and_then(|state| {
            state.trust_record.map(|record| TrustRecord {
                host_flavor: config.flavor,
                extension_id: record.extension_id,
                extension_name: record.extension_name,
                allowed_providers: record.allowed_providers,
            })
        }));
    let vault = Vault::new(Box::new(KeyringSecretStore::new(config.flavor)));
    let host_id = config.flavor.host_id();
    let wrapper_path = host_wrapper_dir().join(host_id);

    let providers = vault
        .providers()
        .into_iter()
        .map(|provider| StatusProvider {
            id: provider.id.to_string(),
            display_name: provider.display_name.to_string(),
            configured: provider.configured,
        })
        .collect();

    let manifests = supported_browsers()
        .iter()
        .map(|browser| {
            let manifest_path = browser_dir(browser).join(format!("{host_id}.json"));
            let browser_root = manifest_path
                .parent()
                .and_then(|path| path.parent())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| browser_dir(browser));
            StatusManifest {
                browser: (*browser).to_string(),
                manifest_path: manifest_path.display().to_string(),
                present: manifest_path.exists(),
                browser_root_present: browser_root.exists(),
            }
        })
        .collect();

    Ok(StatusOutput {
        flavor: config.flavor.as_str().to_string(),
        host_id: host_id.to_string(),
        state_path: state_store.path().display().to_string(),
        wrapper_path: wrapper_path.display().to_string(),
        wrapper_present: wrapper_path.exists(),
        pairing: persisted_pairing.map(|record| StatusPairing {
            extension_id: record.extension_id,
            extension_name: record.extension_name,
            allowed_providers: record.allowed_providers,
        }),
        providers,
        manifests,
        standalone_admin_hint: "cargo run --bin keystone -- serve".to_string(),
    })
}

fn print_status_human(status: &StatusOutput) {
    println!("Keystone status");
    println!("Flavor: {}", status.flavor);
    println!("Host ID: {}", status.host_id);
    println!("State file: {}", status.state_path);
    println!("Wrapper: {}", status.wrapper_path);
    println!(
        "Wrapper present: {}",
        if status.wrapper_present { "yes" } else { "no" }
    );

    match &status.pairing {
        Some(record) => {
            println!("Paired extension: {}", record.extension_name);
            println!("Extension ID: {}", record.extension_id);
            println!(
                "Allowed providers: {}",
                if record.allowed_providers.is_empty() {
                    "none".to_string()
                } else {
                    record.allowed_providers.join(", ")
                }
            );
        }
        None => println!("Paired extension: none"),
    }

    println!("Providers:");
    for provider in &status.providers {
        println!(
            "  - {} ({}) configured={}",
            provider.display_name, provider.id, provider.configured
        );
    }

    println!("Installed browser manifests:");
    for manifest in &status.manifests {
        println!(
            "  - {}: {}{}",
            manifest.browser,
            manifest.manifest_path,
            if manifest.present { " [present]" } else { " [missing]" }
        );
    }

    println!("Standalone admin: {}", status.standalone_admin_hint);
}

fn run_detect() -> Result<(), KeystoneError> {
    let mut found = false;
    for browser in supported_browsers() {
        let dir = browser_dir(browser);
        let root = dir.parent().unwrap_or(&dir);
        if root.exists() {
            println!("{}\t{}", browser, dir.display());
            found = true;
        }
    }

    if !found {
        return Err(KeystoneError::Internal(
            "no supported Chromium-family browser config directories detected".to_string(),
        ));
    }

    Ok(())
}

fn run_install(args: &[String]) -> Result<(), KeystoneError> {
    if args.len() < 3 || args.len() > 4 {
        print_usage();
        return Err(KeystoneError::Internal(
            "usage: keystone install <browser|all> <dev|beta|prod> <extension-id> [binary-path]"
                .to_string(),
        ));
    }

    let target = &args[0];
    let flavor = parse_flavor(&args[1]).ok_or_else(|| {
        KeystoneError::Internal(format!("invalid flavor: {}", args[1]))
    })?;
    let extension_id = &args[2];
    let binary_path = if let Some(path) = args.get(3) {
        PathBuf::from(path)
    } else {
        env::current_exe()?
    };

    if !binary_path.exists() {
        return Err(KeystoneError::Internal(format!(
            "binary path does not exist: {}",
            binary_path.display()
        )));
    }

    if target == "all" {
        let mut installed_any = false;
        for browser in supported_browsers() {
            let dir = browser_dir(browser);
            let root = dir.parent().unwrap_or(&dir);
            if root.exists() {
                install_one(browser, flavor, extension_id, &binary_path)?;
                installed_any = true;
            }
        }
        if !installed_any {
            return Err(KeystoneError::Internal(
                "no supported installed browsers detected for target=all".to_string(),
            ));
        }
        return Ok(());
    }

    if !supported_browsers().contains(&target.as_str()) {
        return Err(KeystoneError::Internal(format!(
            "unsupported browser: {target}"
        )));
    }

    install_one(target, flavor, extension_id, &binary_path)
}

fn install_one(
    browser: &str,
    flavor: HostFlavor,
    extension_id: &str,
    binary_path: &Path,
) -> Result<(), KeystoneError> {
    let manifest_dir = browser_dir(browser);
    fs::create_dir_all(&manifest_dir)?;

    let host_id = flavor.host_id();
    let manifest_path = manifest_dir.join(format!("{host_id}.json"));
    let wrapper_dir = host_wrapper_dir();
    fs::create_dir_all(&wrapper_dir)?;
    let wrapper_path = wrapper_dir.join(host_id);

    let wrapper = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nexport KEYSTONE_FLAVOR=\"{}\"\nexec \"{}\" \"$@\"\n",
        flavor.as_str(),
        binary_path.display()
    );
    fs::write(&wrapper_path, wrapper)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&wrapper_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper_path, perms)?;
    }

    let manifest = NativeHostManifest::for_flavor(
        flavor,
        &wrapper_path.display().to_string(),
        extension_id,
    );
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_json)?;

    println!("installed {}: {}", browser, manifest_path.display());
    println!("wrapper: {}", wrapper_path.display());
    Ok(())
}

fn host_wrapper_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/keystone/native-hosts")
}

fn browser_dir(browser: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    match browser {
        "chrome" => home.join(".config/google-chrome/NativeMessagingHosts"),
        "chromium" => home.join(".config/chromium/NativeMessagingHosts"),
        "brave" => home.join(".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        "opera" => home.join(".config/opera/NativeMessagingHosts"),
        "vivaldi" => home.join(".config/vivaldi/NativeMessagingHosts"),
        other => home.join(format!(".config/{other}/NativeMessagingHosts")),
    }
}

fn supported_browsers() -> &'static [&'static str] {
    &["chrome", "chromium", "brave", "opera", "vivaldi"]
}

fn parse_flavor(value: &str) -> Option<HostFlavor> {
    match value {
        "dev" => Some(HostFlavor::Dev),
        "beta" => Some(HostFlavor::Beta),
        "prod" => Some(HostFlavor::Prod),
        _ => None,
    }
}

fn print_usage() {
    eprintln!(
        "usage:\n  keystone serve\n  keystone status [--json]\n  keystone detect\n  keystone install <browser|all> <dev|beta|prod> <extension-id> [binary-path]"
    );
}

#[derive(Debug, Serialize)]
struct StatusOutput {
    flavor: String,
    host_id: String,
    state_path: String,
    wrapper_path: String,
    wrapper_present: bool,
    pairing: Option<StatusPairing>,
    providers: Vec<StatusProvider>,
    manifests: Vec<StatusManifest>,
    standalone_admin_hint: String,
}

#[derive(Debug, Serialize)]
struct StatusPairing {
    extension_id: String,
    extension_name: String,
    allowed_providers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct StatusProvider {
    id: String,
    display_name: String,
    configured: bool,
}

#[derive(Debug, Serialize)]
struct StatusManifest {
    browser: String,
    manifest_path: String,
    present: bool,
    browser_root_present: bool,
}
