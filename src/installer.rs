use std::fs;
use std::path::{Path, PathBuf};

use crate::config::HostFlavor;
use crate::error::KeystoneError;
use crate::manifest::NativeHostManifest;

pub fn supported_browsers() -> &'static [&'static str] {
    #[cfg(target_os = "linux")]
    {
        &["chrome", "chromium", "brave", "opera", "vivaldi"]
    }
    #[cfg(target_os = "macos")]
    {
        &["chrome", "chromium", "brave", "vivaldi"]
    }
    #[cfg(target_os = "windows")]
    {
        &["chrome", "chromium", "brave", "vivaldi"]
    }
}

pub fn host_wrapper_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("keystone/native-hosts")
}

pub fn wrapper_path_for_host(host_id: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        host_wrapper_dir().join(format!("{host_id}.cmd"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        host_wrapper_dir().join(host_id)
    }
}

pub fn browser_manifest_path(browser: &str, host_id: &str) -> PathBuf {
    browser_manifest_dir(browser).join(format!("{host_id}.json"))
}

pub fn browser_root_dir(browser: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    #[cfg(target_os = "linux")]
    {
        return match browser {
            "chrome" => home.join(".config/google-chrome"),
            "chromium" => home.join(".config/chromium"),
            "brave" => home.join(".config/BraveSoftware/Brave-Browser"),
            "opera" => home.join(".config/opera"),
            "vivaldi" => home.join(".config/vivaldi"),
            other => home.join(format!(".config/{other}")),
        };
    }
    #[cfg(target_os = "macos")]
    {
        return match browser {
            "chrome" => home.join("Library/Application Support/Google/Chrome"),
            "chromium" => home.join("Library/Application Support/Chromium"),
            "brave" => home.join("Library/Application Support/BraveSoftware/Brave-Browser"),
            "vivaldi" => home.join("Library/Application Support/Vivaldi"),
            other => home.join(format!("Library/Application Support/{other}")),
        };
    }
    #[cfg(target_os = "windows")]
    {
        let local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        return match browser {
            "chrome" => local.join("Google/Chrome/User Data"),
            "chromium" => local.join("Chromium/User Data"),
            "brave" => local.join("BraveSoftware/Brave-Browser/User Data"),
            "vivaldi" => local.join("Vivaldi/User Data"),
            other => local.join(format!("{other}/User Data")),
        };
    }
}

pub fn install_one(
    browser: &str,
    flavor: HostFlavor,
    extension_id: &str,
    binary_path: &Path,
) -> Result<(), KeystoneError> {
    let manifest_dir = browser_manifest_dir(browser);
    fs::create_dir_all(&manifest_dir)?;

    let host_id = flavor.host_id();
    let manifest_path = browser_manifest_path(browser, host_id);
    let wrapper_dir = host_wrapper_dir();
    fs::create_dir_all(&wrapper_dir)?;
    let wrapper_path = wrapper_path_for_host(host_id);

    #[cfg(target_os = "windows")]
    {
        let wrapper = format!(
            "@echo off\r\nset KEYSTONE_FLAVOR={}\r\n\"{}\" %*\r\n",
            flavor.as_str(),
            binary_path.display()
        );
        fs::write(&wrapper_path, wrapper)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
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
    }

    let manifest = NativeHostManifest::for_flavor(
        flavor,
        &wrapper_path.display().to_string(),
        extension_id,
    );
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, manifest_json)?;

    #[cfg(target_os = "windows")]
    {
        install_windows_registry(browser, host_id, &manifest_path)?;
    }

    println!("installed {}: {}", browser, manifest_path.display());
    println!("wrapper: {}", wrapper_path.display());
    Ok(())
}

pub fn remove_one(browser: &str, host_id: &str) -> Result<bool, KeystoneError> {
    let manifest_path = browser_manifest_path(browser, host_id);
    let mut removed = false;
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)?;
        removed = true;
    }

    #[cfg(target_os = "windows")]
    {
        remove_windows_registry(browser, host_id)?;
    }

    Ok(removed)
}

fn browser_manifest_dir(browser: &str) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        return browser_root_dir(browser).join("NativeMessagingHosts");
    }
    #[cfg(target_os = "macos")]
    {
        return browser_root_dir(browser).join("NativeMessagingHosts");
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(format!("keystone/native-host-manifests/{browser}"))
    }
}

#[cfg(target_os = "windows")]
fn browser_registry_key(browser: &str, host_id: &str) -> Option<String> {
    let vendor = match browser {
        "chrome" => r"HKCU\Software\Google\Chrome\NativeMessagingHosts",
        "chromium" => r"HKCU\Software\Chromium\NativeMessagingHosts",
        "brave" => r"HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts",
        "vivaldi" => r"HKCU\Software\Vivaldi\NativeMessagingHosts",
        _ => return None,
    };
    Some(format!(r"{vendor}\{host_id}"))
}

#[cfg(target_os = "windows")]
fn install_windows_registry(browser: &str, host_id: &str, manifest_path: &Path) -> Result<(), KeystoneError> {
    let key = browser_registry_key(browser, host_id)
        .ok_or_else(|| KeystoneError::Internal(format!("unsupported browser for windows install: {browser}")))?;
    let status = std::process::Command::new("reg")
        .args([
            "ADD",
            &key,
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            &manifest_path.display().to_string(),
            "/f",
        ])
        .status()?;
    if !status.success() {
        return Err(KeystoneError::Internal(format!(
            "failed to register native host manifest for {browser}"
        )));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_windows_registry(browser: &str, host_id: &str) -> Result<(), KeystoneError> {
    let Some(key) = browser_registry_key(browser, host_id) else {
        return Ok(());
    };
    let status = std::process::Command::new("reg")
        .args(["DELETE", &key, "/f"])
        .status()?;
    if !status.success() {
        return Ok(());
    }
    Ok(())
}
