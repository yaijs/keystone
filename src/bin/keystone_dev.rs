use std::env;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use chrono::Utc;
use keystone::config::{HostFlavor, RuntimeConfig};
use keystone::manifest::NativeHostManifest;
use keystone::state_store::StateStore;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use uuid::Uuid;

fn main() {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        std::process::exit(1);
    };

    match command.as_str() {
        "hello" => print_json(request("bridge.hello", json!({
            "protocol_version": "1.0",
            "extension_name": "Y/TXT Dev"
        }))),
        "pair" => {
            let providers: Vec<String> = args.collect();
            let requested_providers = if providers.is_empty() {
                vec!["openai".to_string()]
            } else {
                providers
            };
            print_json(request("bridge.pair", json!({
                "extension_name": "Y/TXT Dev",
                "requested_providers": requested_providers
            })));
        }
        "set-secret" => {
            let Some(provider) = args.next() else {
                eprintln!("missing provider");
                std::process::exit(1);
            };
            let Some(secret) = args.next() else {
                eprintln!("missing secret");
                std::process::exit(1);
            };
            print_json(request("vault.set_secret", json!({
                "provider": provider,
                "secret": secret
            })));
        }
        "open-session" => {
            let provider = args.next().unwrap_or_else(|| "openai".to_string());
            print_json(request("llm.open_session", json!({
                "provider_id": provider,
                "operation": "chat.completions"
            })));
        }
        "status" => print_json(request("bridge.status", json!({}))),
        "flow-example" => print_flow_example(),
        "manifest" => print_manifest(args),
        "runtime-info" => print_runtime_info(),
        "vault-info" => print_vault_info(),
        "smoke" => run_smoke(),
        "smoke-persist" => run_smoke_persist(),
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

fn request(method: &str, params: Value) -> Value {
    json!({
        "id": Uuid::new_v4().to_string(),
        "method": method,
        "params": params
    })
}

fn print_json(value: Value) {
    println!("{}", serde_json::to_string_pretty(&value).expect("json serialization should work"));
}

fn print_flow_example() {
    let example = json!({
        "steps": [
            {
                "command": "cargo run --bin keystone-dev -- hello",
                "purpose": "Generate bridge.hello request JSON"
            },
            {
                "command": "cargo run --bin keystone-dev -- pair openai",
                "purpose": "Generate bridge.pair request JSON"
            },
            {
                "command": "cargo run --bin keystone-dev -- set-secret openai sk-...",
                "purpose": "Generate vault.set_secret request JSON"
            },
            {
                "command": "cargo run --bin keystone-dev -- open-session openai",
                "purpose": "Generate llm.open_session request JSON"
            }
        ],
        "timestamp": Utc::now()
    });
    print_json(example);
}

fn print_runtime_info() {
    let config = RuntimeConfig::load();
    let state_path = StateStore::new(config.flavor)
        .ok()
        .map(|store| store.path().display().to_string());
    let info = json!({
        "flavor": match config.flavor {
            HostFlavor::Dev => "dev",
            HostFlavor::Beta => "beta",
            HostFlavor::Prod => "prod"
        },
        "host_id": config.flavor.host_id(),
        "extension_origin": config.extension_origin,
        "extension_id": config.extension_id,
        "state_path": state_path
    });
    print_json(info);
}

fn print_vault_info() {
    let config = RuntimeConfig::load();
    let use_in_memory = std::env::var("KEYSTONE_IN_MEMORY_VAULT")
        .ok()
        .as_deref()
        == Some("1");

    let vault = if use_in_memory {
        keystone::vault::Vault::new(Box::<keystone::vault::InMemorySecretStore>::default())
    } else {
        keystone::vault::Vault::new(Box::new(keystone::vault::KeyringSecretStore::new(
            config.flavor,
        )))
    };

    let providers = vault
        .list_provider_info()
        .into_iter()
        .map(|provider| {
            json!({
                "id": provider.id,
                "display_name": provider.display_name,
                "configured": provider.configured
            })
        })
        .collect::<Vec<_>>();

    print_json(json!({
        "flavor": config.flavor.as_str(),
        "host_id": config.flavor.host_id(),
        "backend": if use_in_memory { "in-memory" } else { "keyring" },
        "keyring_service": if use_in_memory { Value::Null } else { json!(config.flavor.keyring_service_name()) },
        "providers": providers
    }));
}

fn print_manifest(mut args: impl Iterator<Item = String>) {
    let flavor = args
        .next()
        .as_deref()
        .and_then(parse_flavor)
        .unwrap_or(HostFlavor::Prod);
    let binary_path = args
        .next()
        .unwrap_or_else(|| keystone_bin_path().display().to_string());
    let extension_id = args
        .next()
        .or_else(|| env::var("KEYSTONE_EXTENSION_ID_OVERRIDE").ok())
        .unwrap_or_else(|| "REPLACE_WITH_EXTENSION_ID".to_string());

    let manifest = NativeHostManifest::for_flavor(flavor, &binary_path, &extension_id);
    print_json(serde_json::to_value(manifest).expect("manifest should serialize"));
}

fn print_usage() {
    eprintln!(
        "usage: keystone-dev <hello|pair|set-secret|open-session|status|flow-example|manifest|runtime-info|vault-info|smoke|smoke-persist>"
    );
}

fn run_smoke() {
    let extension_id =
        env::var("KEYSTONE_EXTENSION_ID_OVERRIDE").unwrap_or_else(|_| "devsmoketestid".to_string());
    let output = run_smoke_flow(&extension_id);
    print_json(output);
}

fn run_smoke_persist() {
    let extension_id =
        env::var("KEYSTONE_EXTENSION_ID_OVERRIDE").unwrap_or_else(|_| "devpersisttestid".to_string());
    let first = run_smoke_flow(&extension_id);
    let second = run_hello_only(&extension_id);

    print_json(json!({
        "first_run": first,
        "second_run_hello": second
    }));
}

fn run_smoke_flow(extension_id: &str) -> Value {
    let origin = format!("chrome-extension://{extension_id}/");
    let keystone_bin = keystone_bin_path();
    let state_dir = smoke_state_dir();

    let mut child = Command::new(&keystone_bin)
        .arg(&origin)
        .env("KEYSTONE_FLAVOR", "dev")
        .env("KEYSTONE_EXTENSION_ID_OVERRIDE", extension_id)
        .env("KEYSTONE_IN_MEMORY_VAULT", "1")
        .env("KEYSTONE_STATE_DIR_OVERRIDE", &state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn keystone host");

    let stdin = child.stdin.as_mut().expect("child stdin missing");
    let stdout = child.stdout.as_mut().expect("child stdout missing");

    let hello = send_nm(stdout, stdin, request("bridge.hello", json!({
        "protocol_version": "1.0",
        "extension_name": "Y/TXT Dev"
    })));
    let pair = send_nm(stdout, stdin, request("bridge.pair", json!({
        "extension_name": "Y/TXT Dev",
        "requested_providers": ["openai"]
    })));
    let set_secret = send_nm(stdout, stdin, request("vault.set_secret", json!({
        "provider": "openai",
        "secret": "sk-test-keystone-dev"
    })));
    let open_session = send_nm(stdout, stdin, request("llm.open_session", json!({
        "provider_id": "openai",
        "operation": "chat.completions"
    })));

    let base_url = open_session["result"]["base_url"]
        .as_str()
        .expect("base_url missing")
        .to_string();
    let token = open_session["result"]["token"]
        .as_str()
        .expect("token missing")
        .to_string();

    let http = Client::new()
        .get(format!("{base_url}/health"))
        .bearer_auth(token)
        .send()
        .expect("health request failed");
    let health_status = http.status().as_u16();
    let health_body: Value = http.json().expect("health body should be json");

    let _ = child.kill();
    let _ = child.wait();

    json!({
        "hello": hello,
        "pair": pair,
        "set_secret": set_secret,
        "open_session": open_session,
        "health_status": health_status,
        "health_body": health_body,
        "state_dir": state_dir
    })
}

fn run_hello_only(extension_id: &str) -> Value {
    let origin = format!("chrome-extension://{extension_id}/");
    let keystone_bin = keystone_bin_path();
    let state_dir = smoke_state_dir();

    let mut child = Command::new(&keystone_bin)
        .arg(&origin)
        .env("KEYSTONE_FLAVOR", "dev")
        .env("KEYSTONE_EXTENSION_ID_OVERRIDE", extension_id)
        .env("KEYSTONE_IN_MEMORY_VAULT", "1")
        .env("KEYSTONE_STATE_DIR_OVERRIDE", &state_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn keystone host");

    let stdin = child.stdin.as_mut().expect("child stdin missing");
    let stdout = child.stdout.as_mut().expect("child stdout missing");
    let hello = send_nm(stdout, stdin, request("bridge.hello", json!({
        "protocol_version": "1.0",
        "extension_name": "Y/TXT Dev"
    })));

    let _ = child.kill();
    let _ = child.wait();

    hello
}

fn smoke_state_dir() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join(".tmp/keystone-smoke-state").display().to_string()
}

fn parse_flavor(value: &str) -> Option<HostFlavor> {
    match value {
        "dev" => Some(HostFlavor::Dev),
        "beta" => Some(HostFlavor::Beta),
        "prod" => Some(HostFlavor::Prod),
        _ => None,
    }
}

fn send_nm<R: Read, W: Write>(stdout: &mut R, stdin: &mut W, value: Value) -> Value {
    let payload = serde_json::to_vec(&value).expect("request should serialize");
    let len = (payload.len() as u32).to_le_bytes();
    stdin.write_all(&len).expect("write len failed");
    stdin.write_all(&payload).expect("write payload failed");
    stdin.flush().expect("flush failed");

    let mut len_buf = [0_u8; 4];
    stdout.read_exact(&mut len_buf).expect("read len failed");
    let response_len = u32::from_le_bytes(len_buf) as usize;
    let mut response = vec![0_u8; response_len];
    stdout
        .read_exact(&mut response)
        .expect("read response failed");
    serde_json::from_slice(&response).expect("response should be valid json")
}

fn keystone_bin_path() -> PathBuf {
    if let Ok(value) = env::var("KEYSTONE_BIN") {
        return PathBuf::from(value);
    }

    build_keystone_binary();

    let current = env::current_exe().expect("current exe path missing");
    current
        .parent()
        .expect("bin directory missing")
        .join("keystone")
}

fn build_keystone_binary() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("keystone")
        .current_dir(manifest_dir)
        .status()
        .expect("failed to invoke cargo build for keystone");

    if !status.success() {
        eprintln!("failed to build keystone binary for smoke harness");
        std::process::exit(1);
    }
}
