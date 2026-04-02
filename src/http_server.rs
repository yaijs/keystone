use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use axum::{
    extract::Path,
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::net::TcpListener;

use crate::app::AppState;
use crate::error::KeystoneError;
use crate::forwarder::{forward_chat_completions, forward_messages};
use crate::installer::{
    browser_manifest_path, browser_root_dir, install_one, remove_one, supported_browsers,
    wrapper_path_for_host,
};
use crate::pairing::TrustRecord;

pub async fn bind_localhost(state: AppState) -> Result<SocketAddr, KeystoneError> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let app = Router::new()
        .route("/health", get(health))
        .route("/admin", get(admin_ui))
        .route("/admin/api/status", get(admin_status))
        .route("/admin/api/install", post(admin_install_detected))
        .route("/admin/api/install/{browser}", post(admin_install_browser))
        .route("/admin/api/install", axum::routing::delete(admin_remove_all))
        .route("/admin/api/secrets", post(admin_set_secret))
        .route("/admin/api/secrets/{provider}", axum::routing::delete(admin_delete_secret))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/messages", post(messages))
        .with_state(state);

    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("http server exited: {err}");
        }
    });

    Ok(addr)
}

async fn health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    let _session = authorize_any(&state, &headers).await?;
    Ok(Json(json!({
        "status": "ok",
        "host_version": crate::protocol::HOST_VERSION
    })))
}

async fn admin_ui() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Keystone Admin</title>
  <style>
    :root {
      --bg: #0b1220;
      --card: #121c2e;
      --muted: #94a3b8;
      --text: #e2e8f0;
      --border: #23304a;
      --accent: #38bdf8;
      --danger: #f87171;
      --success: #34d399;
    }
    body {
      margin: 0;
      font-family: ui-sans-serif, system-ui, sans-serif;
      background: radial-gradient(circle at top, #16233b, var(--bg) 60%);
      color: var(--text);
    }
    main {
      max-width: 920px;
      margin: 0 auto;
      padding: 32px 20px 56px;
    }
    h1, h2 { margin: 0 0 12px; }
    p { color: var(--muted); }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
      gap: 16px;
      margin-top: 18px;
    }
    .card {
      background: rgba(18, 28, 46, 0.92);
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 16px;
    }
    .pill {
      display: inline-block;
      padding: 4px 10px;
      border-radius: 999px;
      border: 1px solid var(--border);
      color: var(--muted);
      font-size: 12px;
    }
    .ok { color: var(--success); }
    .bad { color: var(--danger); }
    .providers { display: grid; gap: 12px; }
    .browser-list { display: grid; gap: 12px; }
    .browser {
      border-top: 1px solid var(--border);
      padding-top: 12px;
    }
    .provider { border-top: 1px solid var(--border); padding-top: 12px; }
    .banner {
      margin-top: 16px;
      background: rgba(56, 189, 248, 0.08);
      border: 1px solid rgba(56, 189, 248, 0.35);
      border-radius: 14px;
      padding: 14px 16px;
    }
    .banner strong {
      display: block;
      margin-bottom: 6px;
    }
    .warning-note {
      color: #fca5a5;
      margin: 0 0 10px;
    }
    label { display: block; font-size: 13px; color: var(--muted); margin-bottom: 6px; }
    input {
      width: 100%;
      box-sizing: border-box;
      padding: 10px 12px;
      border-radius: 10px;
      border: 1px solid var(--border);
      background: #0b1220;
      color: var(--text);
    }
    .actions { display: flex; gap: 8px; margin-top: 8px; }
    .section-actions {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      margin: 10px 0 16px;
    }
    button {
      border: 1px solid var(--border);
      background: #18263f;
      color: var(--text);
      padding: 9px 12px;
      border-radius: 10px;
      cursor: pointer;
    }
    button:hover { border-color: var(--accent); }
    button:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
    .danger:hover { border-color: var(--danger); }
    code {
      background: #0b1220;
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 3px 8px;
    }
    #message { margin-top: 14px; min-height: 22px; color: var(--muted); }
  </style>
</head>
<body>
  <main>
    <h1>Keystone Admin</h1>
    <p>Local status, pairing diagnostics, and provider secret management for the current Keystone flavor.</p>
    <section class="banner">
      <strong id="setup-title">Checking setup...</strong>
      <div id="setup-summary"></div>
    </section>
    <div class="grid">
      <section class="card">
        <h2>Runtime</h2>
        <div id="runtime"></div>
      </section>
      <section class="card">
        <h2>Pairing</h2>
        <div id="pairing"></div>
      </section>
    </div>
    <section class="card" style="margin-top: 16px;">
      <h2>Browser Install Status</h2>
      <p class="warning-note">Remove All deletes the Native Messaging install state for this Keystone flavor. Extension-side actions such as Open Keystone Admin will stop working until you install again.</p>
      <div class="section-actions">
        <button id="install-detected-btn">Install For Detected Browsers</button>
        <button id="remove-all-btn" class="danger">Remove All</button>
      </div>
      <div id="browsers" class="browser-list"></div>
    </section>
    <section class="card" style="margin-top: 16px;">
      <h2>Providers</h2>
      <div id="providers" class="providers"></div>
      <div id="message"></div>
    </section>
  </main>
  <script>
    const runtimeEl = document.getElementById('runtime');
    const pairingEl = document.getElementById('pairing');
    const browsersEl = document.getElementById('browsers');
    const providersEl = document.getElementById('providers');
    const messageEl = document.getElementById('message');
    const setupTitleEl = document.getElementById('setup-title');
    const setupSummaryEl = document.getElementById('setup-summary');
    const installDetectedBtn = document.getElementById('install-detected-btn');
    const removeAllBtn = document.getElementById('remove-all-btn');
    const adminToken = new URLSearchParams(window.location.search).get('token') || '';

    function setMessage(text, isError = false) {
      messageEl.textContent = text;
      messageEl.className = isError ? 'bad' : 'ok';
    }

    function adminHeaders(extra = {}) {
      return adminToken
        ? { ...extra, authorization: `Bearer ${adminToken}` }
        : { ...extra };
    }

    async function adminFetch(url, init = {}) {
      return fetch(url, {
        ...init,
        headers: adminHeaders(init.headers || {}),
      });
    }

    function setSetupBanner(data) {
      const installed = data.browser_manifests.filter((browser) => browser.present);
      const missingWithRoot = data.browser_manifests.filter((browser) => !browser.present && browser.browser_root_present);
      const configuredProviders = data.providers.filter((provider) => provider.configured);

      if (!installed.length && missingWithRoot.length) {
        setupTitleEl.textContent = 'Install Keystone into your browser';
        setupSummaryEl.innerHTML = `<p>Native Messaging is not installed for this flavor yet. The fastest next step is to copy and run the command for <code>${missingWithRoot[0].browser}</code> below.</p>`;
        return;
      }

      if (installed.length && !data.pairing) {
        setupTitleEl.textContent = 'Pair an extension';
        setupSummaryEl.innerHTML = '<p>Keystone is installed for at least one browser, but no paired extension is recorded for this runtime yet. Open your extension and trigger a Keystone-backed action.</p>';
        return;
      }

      if (installed.length && data.pairing && !configuredProviders.length) {
        setupTitleEl.textContent = 'Add a provider secret';
        setupSummaryEl.innerHTML = '<p>Browser install and pairing are in place. The next step is to store at least one provider secret below so requests can be tunneled.</p>';
        return;
      }

      setupTitleEl.textContent = 'Keystone is ready';
      setupSummaryEl.innerHTML = '<p>Browser install, pairing, and provider configuration look healthy for this runtime.</p>';
    }

    async function loadStatus() {
      const response = await adminFetch('/admin/api/status');
      const data = await response.json();

      runtimeEl.innerHTML = `
        <p><span class="pill">${data.flavor}</span></p>
        <p>Host ID: <code>${data.host_id}</code></p>
        <p>Version: <code>${data.host_version}</code></p>
        <p>Uptime: <code>${data.uptime_seconds}s</code></p>
        <p>Active sessions: <code>${data.active_sessions}</code></p>
        <p>State file: <code>${data.state_path}</code></p>
        <p>Binary: <code>${data.binary_path}</code></p>
      `;

      pairingEl.innerHTML = data.pairing
        ? `
          <p class="ok">Paired</p>
          <p>Extension: <code>${data.pairing.extension_name}</code></p>
          <p>Extension ID: <code>${data.pairing.extension_id}</code></p>
          <p>Allowed providers: <code>${data.pairing.allowed_providers.join(', ') || 'none'}</code></p>
        `
        : `<p class="bad">No paired extension recorded for this flavor/runtime.</p>`;

      browsersEl.innerHTML = '';
      for (const browser of data.browser_manifests) {
        const section = document.createElement('section');
        section.className = 'browser';
        section.innerHTML = `
          <div style="display:flex; justify-content:space-between; gap: 12px; align-items:baseline;">
            <strong>${browser.browser}</strong>
            <span class="${browser.present ? 'ok' : 'bad'}">${browser.present ? 'installed' : 'missing'}</span>
          </div>
          <p>Config dir: <code>${browser.browser_root_path}</code> ${browser.browser_root_present ? '' : '<span class="bad">(missing)</span>'}</p>
          <p>Manifest: <code>${browser.manifest_path}</code></p>
          <p>Install command:</p>
          <p><code>${browser.install_command}</code></p>
          <div class="actions">
            <button data-browser="${browser.browser}" data-action="install-browser" ${browser.browser_root_present ? '' : 'disabled'}>Install Here</button>
            <button data-command="${browser.install_command.replace(/"/g, '&quot;')}" data-browser="${browser.browser}" data-action="copy-install">Copy Install Command</button>
          </div>
        `;
        browsersEl.appendChild(section);
      }

      setSetupBanner(data);

      providersEl.innerHTML = '';
      for (const provider of data.providers) {
        const section = document.createElement('section');
        section.className = 'provider';
        section.innerHTML = `
          <div style="display:flex; justify-content:space-between; gap: 12px; align-items:baseline;">
            <strong>${provider.display_name}</strong>
            <span class="${provider.configured ? 'ok' : 'bad'}">${provider.configured ? 'configured' : 'missing'}</span>
          </div>
          <p>ID: <code>${provider.id}</code></p>
          <p>Upstream: <code>${provider.base_url}</code></p>
          <label for="secret-${provider.id}">Replace secret</label>
          <input id="secret-${provider.id}" type="password" placeholder="Paste a new secret for ${provider.id}" />
          <div class="actions">
            <button data-provider="${provider.id}" data-action="save">Save Secret</button>
            <button data-provider="${provider.id}" data-action="delete" class="danger">Delete Secret</button>
          </div>
        `;
        providersEl.appendChild(section);
      }

      browsersEl.querySelectorAll('button[data-action="copy-install"]').forEach((button) => {
        button.addEventListener('click', async () => {
          const command = button.getAttribute('data-command');
          const browser = button.getAttribute('data-browser') || 'browser';
          if (!command) return;

          try {
            await navigator.clipboard.writeText(command);
            setMessage(`Copied install command for ${browser}.`);
          } catch (error) {
            setMessage(`Failed to copy install command for ${browser}: ${error.message || String(error)}`, true);
          }
        });
      });

      browsersEl.querySelectorAll('button[data-action="install-browser"]').forEach((button) => {
        button.addEventListener('click', async () => {
          const browser = button.getAttribute('data-browser');
          if (!browser) return;

          try {
            const resp = await adminFetch(`/admin/api/install/${browser}`, { method: 'POST' });
            const result = await resp.json();
            if (!resp.ok) throw new Error(result.error || result.message || 'install failed');
            setMessage(`Installed Keystone for ${browser}.`);
            await loadStatus();
          } catch (error) {
            setMessage(`Failed to install for ${browser}: ${error.message || String(error)}`, true);
          }
        });
      });

      providersEl.querySelectorAll('button').forEach((button) => {
        button.addEventListener('click', async () => {
          const provider = button.getAttribute('data-provider');
          const action = button.getAttribute('data-action');
          if (!provider) return;

          try {
            if (action === 'save') {
              const input = document.getElementById(`secret-${provider}`);
              const secret = input && 'value' in input ? input.value.trim() : '';
              if (!secret) {
                setMessage(`Enter a secret for ${provider} first.`, true);
                return;
              }
              const resp = await adminFetch('/admin/api/secrets', {
                method: 'POST',
                headers: { 'content-type': 'application/json' },
                body: JSON.stringify({ provider, secret })
              });
              const result = await resp.json();
              if (!resp.ok) throw new Error(result.error || result.message || 'save failed');
              setMessage(`Stored secret for ${provider}.`);
              input.value = '';
            } else {
              const resp = await adminFetch(`/admin/api/secrets/${provider}`, { method: 'DELETE' });
              const result = await resp.json();
              if (!resp.ok) throw new Error(result.error || result.message || 'delete failed');
              setMessage(`Deleted secret for ${provider}.`);
            }
            await loadStatus();
          } catch (error) {
            setMessage(error.message || String(error), true);
          }
        });
      });
    }

    installDetectedBtn.addEventListener('click', async () => {
      try {
        const resp = await adminFetch('/admin/api/install', { method: 'POST' });
        const result = await resp.json();
        if (!resp.ok) throw new Error(result.error || result.message || 'install failed');
        const installed = Array.isArray(result.installed) ? result.installed : [];
        const skipped = Array.isArray(result.skipped) ? result.skipped : [];
        const parts = [];
        if (installed.length) parts.push(`installed: ${installed.join(', ')}`);
        if (skipped.length) parts.push(`skipped: ${skipped.join(', ')}`);
        setMessage(parts.length ? `Detected browser install complete (${parts.join(' | ')}).` : 'No supported browsers were changed.');
        await loadStatus();
      } catch (error) {
        setMessage(`Failed to install for detected browsers: ${error.message || String(error)}`, true);
      }
    });

    removeAllBtn.addEventListener('click', async () => {
      const confirmed = window.confirm('Remove all Native Messaging manifests and the wrapper for this Keystone flavor? Extension-side Keystone actions will stop working until you install again.');
      if (!confirmed) return;

      try {
        const resp = await adminFetch('/admin/api/install', { method: 'DELETE' });
        const result = await resp.json();
        if (!resp.ok) throw new Error(result.error || result.message || 'remove failed');
        const removed = Array.isArray(result.removed) ? result.removed : [];
        const wrapperRemoved = result.wrapper_removed === true;
        const parts = [];
        if (removed.length) parts.push(`manifests removed: ${removed.join(', ')}`);
        if (wrapperRemoved) parts.push('wrapper removed');
        if (!removed.length && !wrapperRemoved) parts.push('nothing was installed for this flavor');
        setMessage(`Keystone install state cleared (${parts.join(' | ')}).`);
        await loadStatus();
      } catch (error) {
        setMessage(`Failed to remove Keystone install state: ${error.message || String(error)}`, true);
      }
    });

    loadStatus().catch((error) => setMessage(error.message || String(error), true));
  </script>
</body>
</html>"#,
    )
}

#[derive(Debug, Serialize)]
struct AdminStatus {
    flavor: String,
    host_id: String,
    host_version: &'static str,
    uptime_seconds: u64,
    state_path: String,
    binary_path: String,
    extension_id_hint: String,
    active_sessions: usize,
    pairing: Option<AdminPairing>,
    browser_manifests: Vec<AdminBrowserManifest>,
    providers: Vec<AdminProvider>,
}

#[derive(Debug, Serialize)]
struct AdminPairing {
    extension_id: String,
    extension_name: String,
    allowed_providers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AdminProvider {
    id: String,
    display_name: String,
    base_url: String,
    configured: bool,
}

#[derive(Debug, Serialize)]
struct AdminBrowserManifest {
    browser: String,
    browser_root_path: String,
    manifest_path: String,
    browser_root_present: bool,
    present: bool,
    install_command: String,
}

#[derive(Debug, Deserialize)]
struct SecretUpdateRequest {
    provider: String,
    secret: String,
}

#[derive(Debug, Serialize)]
struct AdminInstallResult {
    ok: bool,
    installed: Vec<String>,
    skipped: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AdminRemoveResult {
    ok: bool,
    removed: Vec<String>,
    wrapper_removed: bool,
}

async fn admin_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminStatus>, HttpError> {
    authorize_admin(&state, &headers)?;
    let pairing = state
        .pairing
        .lock()
        .await
        .current_record()
        .map(|record| pairing_to_admin(record));
    let active_sessions = state.sessions.lock().await.count();
    let vault = state.vault.lock().await;
    let host_id = state.config.flavor.host_id().to_string();
    let extension_id_hint = pairing
        .as_ref()
        .map(|record| record.extension_id.clone())
        .unwrap_or_else(|| state.extension_id_seen.clone());
    let binary_path = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "keystone".to_string());
    let browser_manifests = supported_browsers()
        .iter()
        .map(|browser| {
            let manifest_path = browser_manifest_path(browser, &host_id);
            let browser_root_path = browser_root_dir(browser);

            AdminBrowserManifest {
                browser: (*browser).to_string(),
                browser_root_path: browser_root_path.display().to_string(),
                manifest_path: manifest_path.display().to_string(),
                browser_root_present: browser_root_path.exists(),
                present: manifest_path.exists(),
                install_command: format!(
                    "{} install {} {} {} {}",
                    shell_escape(&binary_path),
                    shell_escape(browser),
                    shell_escape(state.config.flavor.as_str()),
                    shell_escape(&extension_id_hint),
                    shell_escape(&binary_path),
                ),
            }
        })
        .collect();
    let providers = state
        .providers
        .all()
        .iter()
        .map(|provider| AdminProvider {
            id: provider.id.to_string(),
            display_name: provider.display_name.to_string(),
            base_url: provider.base_url.to_string(),
            configured: vault.is_configured(provider.id),
        })
        .collect();

    Ok(Json(AdminStatus {
        flavor: state.config.flavor.as_str().to_string(),
        host_id,
        host_version: crate::protocol::HOST_VERSION,
        uptime_seconds: state.started_at.elapsed().as_secs(),
        state_path: state.state_store.path().display().to_string(),
        binary_path,
        extension_id_hint,
        active_sessions,
        pairing,
        browser_manifests,
        providers,
    }))
}

async fn admin_install_detected(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminInstallResult>, HttpError> {
    authorize_admin(&state, &headers)?;
    let extension_id = current_extension_id_hint(&state).await;
    let binary_path = current_binary_path()?;
    let mut installed = Vec::new();
    let mut skipped = Vec::new();

    for browser in supported_browsers() {
        let root = browser_root_dir(browser);
        if root.exists() {
            install_one(browser, state.config.flavor, &extension_id, &binary_path)
                .map_err(HttpError::Upstream)?;
            installed.push(browser.to_string());
        } else {
            skipped.push(browser.to_string());
        }
    }

    if installed.is_empty() {
        return Err(HttpError::BadRequest(
            "no supported installed browsers detected".to_string(),
        ));
    }

    Ok(Json(AdminInstallResult {
        ok: true,
        installed,
        skipped,
    }))
}

async fn admin_install_browser(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(browser): Path<String>,
) -> Result<Json<AdminInstallResult>, HttpError> {
    authorize_admin(&state, &headers)?;
    if !supported_browsers().contains(&browser.as_str()) {
        return Err(HttpError::BadRequest("unsupported browser".to_string()));
    }

    let root = browser_root_dir(&browser);
    if !root.exists() {
        return Err(HttpError::BadRequest(format!(
            "{browser} config directory not detected on this machine"
        )));
    }

    let extension_id = current_extension_id_hint(&state).await;
    let binary_path = current_binary_path()?;
    install_one(&browser, state.config.flavor, &extension_id, &binary_path)
        .map_err(HttpError::Upstream)?;

    Ok(Json(AdminInstallResult {
        ok: true,
        installed: vec![browser],
        skipped: Vec::new(),
    }))
}

async fn admin_remove_all(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminRemoveResult>, HttpError> {
    authorize_admin(&state, &headers)?;
    let mut removed = Vec::new();
    let host_id = state.config.flavor.host_id();

    for browser in supported_browsers() {
        if remove_one(browser, host_id).map_err(|err| HttpError::Upstream(err))? {
            removed.push(browser.to_string());
        }
    }

    let wrapper_path = wrapper_path_for_host(host_id);
    let mut wrapper_removed = false;
    if wrapper_path.exists() {
        fs::remove_file(wrapper_path)
            .map_err(|err: std::io::Error| HttpError::Upstream(err.into()))?;
        wrapper_removed = true;
    }

    Ok(Json(AdminRemoveResult {
        ok: true,
        removed,
        wrapper_removed,
    }))
}

async fn admin_set_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SecretUpdateRequest>,
) -> Result<Json<Value>, HttpError> {
    authorize_admin(&state, &headers)?;
    if payload.secret.trim().is_empty() {
        return Err(HttpError::BadRequest("secret must not be empty".to_string()));
    }

    let mut vault = state.vault.lock().await;
    if !vault.is_provider_known(&payload.provider) {
        return Err(HttpError::BadRequest("unknown provider".to_string()));
    }

    if vault.set_secret(&payload.provider, &payload.secret) {
        Ok(Json(json!({ "ok": true })))
    } else {
        Err(HttpError::Upstream(KeystoneError::Internal(
            "failed to store secret".to_string(),
        )))
    }
}

async fn admin_delete_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Result<Json<Value>, HttpError> {
    authorize_admin(&state, &headers)?;
    let mut vault = state.vault.lock().await;
    if !vault.is_provider_known(&provider) {
        return Err(HttpError::BadRequest("unknown provider".to_string()));
    }

    if vault.delete_secret(&provider) {
        Ok(Json(json!({ "ok": true })))
    } else {
        Err(HttpError::BadRequest("provider secret not found".to_string()))
    }
}

fn pairing_to_admin(record: TrustRecord) -> AdminPairing {
    AdminPairing {
        extension_id: record.extension_id,
        extension_name: record.extension_name,
        allowed_providers: record.allowed_providers,
    }
}

fn current_binary_path() -> Result<PathBuf, HttpError> {
    std::env::current_exe().map_err(|err| HttpError::Upstream(err.into()))
}

async fn current_extension_id_hint(state: &AppState) -> String {
    state
        .pairing
        .lock()
        .await
        .current_record()
        .map(|record| record.extension_id)
        .unwrap_or_else(|| state.extension_id_seen.clone())
}

fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', r"'\''"))
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), HttpError> {
    let session = authorize(&state, &headers, "chat.completions").await?;
    let (status, body) = forward_chat_completions(&state, &session.provider_id, payload)
        .await
        .map_err(HttpError::Upstream)?;
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((status, Json(body)))
}

async fn messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> Result<(StatusCode, Json<Value>), HttpError> {
    let session = authorize(&state, &headers, "messages").await?;
    let (status, body) = forward_messages(&state, &session.provider_id, payload)
        .await
        .map_err(HttpError::Upstream)?;
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((status, Json(body)))
}

async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    operation: &str,
) -> Result<crate::session::SessionRecord, HttpError> {
    let token = extract_bearer_token(headers).ok_or(HttpError::Unauthorized)?;
    let mut sessions = state.sessions.lock().await;
    sessions
        .validate_token(&token, operation)
        .ok_or(HttpError::Unauthorized)
}

async fn authorize_any(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::session::SessionRecord, HttpError> {
    let token = extract_bearer_token(headers).ok_or(HttpError::Unauthorized)?;
    let mut sessions = state.sessions.lock().await;
    sessions.validate_token_any(&token).ok_or(HttpError::Unauthorized)
}

fn authorize_admin(state: &AppState, headers: &HeaderMap) -> Result<(), HttpError> {
    let token = extract_bearer_token(headers).ok_or(HttpError::Unauthorized)?;
    if token == state.admin_token {
        Ok(())
    } else {
        Err(HttpError::Unauthorized)
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

enum HttpError {
    Unauthorized,
    BadRequest(String),
    Upstream(KeystoneError),
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "missing_or_invalid_session"
                })),
            )
                .into_response(),
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "bad_request",
                    "message": message
                })),
            )
                .into_response(),
            Self::Upstream(err) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": "upstream_failure",
                    "message": err.to_string()
                })),
            )
                .into_response(),
        }
    }
}
