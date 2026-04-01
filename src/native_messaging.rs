use std::io::{Read, Write};
use std::path::PathBuf;

use serde::Serialize;

use crate::app::AppState;
use crate::error::KeystoneError;
use crate::protocol::{
    supported_methods, Capabilities, ErrorCode, ErrorEnvelope, ErrorPayload, HelloParams,
    HelloResult, HOST_VERSION, OpenSessionParams, OpenSessionResult, OkResult, OpenSettingsResult,
    PairParams, PairResult, PairingStatus, ProviderListResult, PROTOCOL_VERSION,
    RequestEnvelope, ResponseEnvelope, StatusResult, VaultDeleteSecretParams,
    VaultSetSecretParams,
};

pub async fn run_native_host(state: AppState) -> Result<(), KeystoneError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();

    loop {
        let request = match read_message(&mut input) {
            Ok(request) => request,
            Err(KeystoneError::Io(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(());
            }
            Err(err) => return Err(err),
        };

        let response = dispatch(&state, request).await;
        write_message(&mut output, &response)?;
    }
}

async fn dispatch(state: &AppState, request: RequestEnvelope) -> serde_json::Value {
    let request_id = request.id.clone();

    match request.method.as_str() {
        "bridge.hello" => match parse_params::<HelloParams>(request_id.clone(), request.params) {
            Ok(params) if params.protocol_version == PROTOCOL_VERSION => {
                success_response(request_id, bridge_hello(state, params).await)
            }
            Ok(_) => error_response(
                request_id,
                ErrorCode::InvalidRequest,
                "unsupported protocol version",
            ),
            Err(value) => value,
        },
        "bridge.pair" => match parse_params::<PairParams>(request_id.clone(), request.params) {
            Ok(params) => match bridge_pair(state, params).await {
                Ok(result) => success_response(request_id, result),
                Err(message) => error_response(request_id, ErrorCode::InternalError, &message),
            },
            Err(value) => value,
        },
        "bridge.status" => success_response(request_id, bridge_status(state).await),
        "bridge.open_settings" => success_response(request_id, bridge_open_settings(state).await),
        "vault.list_providers" => success_response(request_id, vault_list_providers(state).await),
        "vault.set_secret" => match parse_params::<VaultSetSecretParams>(request_id.clone(), request.params) {
            Ok(params) => match vault_set_secret(state, request_id, params).await {
                Ok(value) | Err(value) => value,
            },
            Err(value) => value,
        },
        "vault.delete_secret" => match parse_params::<VaultDeleteSecretParams>(request_id.clone(), request.params) {
            Ok(params) => match vault_delete_secret(state, request_id, params).await {
                Ok(value) | Err(value) => value,
            },
            Err(value) => value,
        },
        "llm.open_session" => match parse_params::<OpenSessionParams>(request_id.clone(), request.params) {
            Ok(params) => match llm_open_session(state, request_id, params).await {
                Ok(value) | Err(value) => value,
            },
            Err(value) => value,
        },
        _ => error_response(
            request_id,
            ErrorCode::MethodNotFound,
            "unknown method",
        ),
    }
}

async fn bridge_hello(state: &AppState, _params: HelloParams) -> HelloResult {
    let pairing_status = {
        let pairing = state.pairing.lock().await;
        pairing.current_status(state.config.flavor, &state.extension_id_seen)
    };

    HelloResult {
        protocol_version: PROTOCOL_VERSION,
        host_version: HOST_VERSION,
        extension_id_seen: state.extension_id_seen.clone(),
        pairing_status,
        supported_methods: supported_methods(),
        capabilities: Capabilities {
            http_data_plane: true,
            responses_api: false,
        },
    }
}

async fn bridge_pair(state: &AppState, params: PairParams) -> Result<PairResult, String> {
    let mut pairing = state.pairing.lock().await;
    if let Some(record) = pairing.get_record(state.config.flavor, &state.extension_id_seen) {
        return Ok(PairResult {
            pairing_status: PairingStatus::Paired,
            allowed_providers: record.allowed_providers,
        });
    }

    let record = pairing.pair_extension(
        state.config.flavor,
        state.extension_id_seen.clone(),
        params.extension_name,
        params.requested_providers,
    );
    state.state_store.save_pairing(&record).map_err(|err| {
        format!(
            "failed to persist pairing to {}: {err}",
            state.state_store.path().display()
        )
    })?;

    Ok(PairResult {
        pairing_status: PairingStatus::Paired,
        allowed_providers: record.allowed_providers,
    })
}

async fn bridge_status(state: &AppState) -> StatusResult {
    let pairing_status = {
        let pairing = state.pairing.lock().await;
        pairing.current_status(state.config.flavor, &state.extension_id_seen)
    };

    let active_sessions = state.sessions.lock().await.count();
    let providers = state.vault.lock().await.list_provider_status();
    let wrapper_path = host_wrapper_dir().join(state.config.flavor.host_id());

    StatusResult {
        host_version: HOST_VERSION,
        uptime_seconds: state.started_at.elapsed().as_secs(),
        pairing_status,
        active_sessions,
        providers,
        admin_ui_url: format!("{}/admin?token={}", state.http_base_url, state.admin_token),
        wrapper_path: wrapper_path.display().to_string(),
        wrapper_present: wrapper_path.exists(),
    }
}

fn host_wrapper_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/keystone/native-hosts")
}

async fn bridge_open_settings(state: &AppState) -> OpenSettingsResult {
    OpenSettingsResult {
        ok: true,
        url: format!("{}/admin?token={}", state.http_base_url, state.admin_token),
    }
}

async fn vault_list_providers(state: &AppState) -> ProviderListResult {
    ProviderListResult {
        providers: state.vault.lock().await.list_provider_info(),
    }
}

async fn vault_set_secret(
    state: &AppState,
    request_id: serde_json::Value,
    params: VaultSetSecretParams,
) -> Result<serde_json::Value, serde_json::Value> {
    ensure_paired(state, &request_id).await?;

    let mut vault = state.vault.lock().await;
    if !vault.is_provider_known(&params.provider) {
        return Err(error_response(
            request_id,
            ErrorCode::ProviderUnknown,
            "unknown provider",
        ));
    }

    if vault.set_secret(&params.provider, &params.secret) {
        Ok(success_response(request_id, OkResult { ok: true }))
    } else {
        Err(error_response(
            request_id,
            ErrorCode::InternalError,
            "failed to store secret",
        ))
    }
}

async fn vault_delete_secret(
    state: &AppState,
    request_id: serde_json::Value,
    params: VaultDeleteSecretParams,
) -> Result<serde_json::Value, serde_json::Value> {
    ensure_paired(state, &request_id).await?;

    let mut vault = state.vault.lock().await;
    if !vault.is_provider_known(&params.provider) {
        return Err(error_response(
            request_id,
            ErrorCode::ProviderUnknown,
            "unknown provider",
        ));
    }

    if vault.delete_secret(&params.provider) {
        Ok(success_response(request_id, OkResult { ok: true }))
    } else {
        Err(error_response(
            request_id,
            ErrorCode::InternalError,
            "failed to delete secret",
        ))
    }
}

async fn llm_open_session(
    state: &AppState,
    request_id: serde_json::Value,
    params: OpenSessionParams,
) -> Result<serde_json::Value, serde_json::Value> {
    let record = ensure_paired(state, &request_id).await?;

    if !record.allowed_providers.contains(&params.provider_id) {
        return Err(error_response(
            request_id,
            ErrorCode::ProviderNotAllowed,
            "provider not allowed for paired extension",
        ));
    }

    let vault = state.vault.lock().await;
    if !vault.is_provider_known(&params.provider_id) {
        return Err(error_response(
            request_id,
            ErrorCode::ProviderUnknown,
            "unknown provider",
        ));
    }
    if !vault.is_configured(&params.provider_id) {
        return Err(error_response(
            request_id,
            ErrorCode::ProviderNotConfigured,
            "provider not configured",
        ));
    }
    drop(vault);

    let session = state.sessions.lock().await.create_session(
        state.extension_id_seen.clone(),
        params.provider_id.clone(),
        params.operation.clone(),
    );

    Ok(success_response(
        request_id,
        OpenSessionResult {
            base_url: state.http_base_url.clone(),
            session_id: session.session_id,
            token: session.token,
            expires_at: session.expires_at,
            provider_id: session.provider_id,
            allowed_operation: session.operation,
        },
    ))
}

async fn ensure_paired(
    state: &AppState,
    request_id: &serde_json::Value,
) -> Result<crate::pairing::TrustRecord, serde_json::Value> {
    let pairing = state.pairing.lock().await;
    pairing
        .get_record(state.config.flavor, &state.extension_id_seen)
        .ok_or_else(|| {
            error_response(
                request_id.clone(),
                ErrorCode::ExtensionNotPaired,
                "extension not paired",
            )
        })
}

fn parse_params<T: serde::de::DeserializeOwned>(
    request_id: serde_json::Value,
    value: serde_json::Value,
) -> Result<T, serde_json::Value> {
    serde_json::from_value(value).map_err(|_| {
        error_response(
            request_id,
            ErrorCode::InvalidRequest,
            "invalid params",
        )
    })
}

fn success_response<T: Serialize>(id: serde_json::Value, result: T) -> serde_json::Value {
    serde_json::to_value(ResponseEnvelope { id, result }).expect("response should serialize")
}

fn error_response(id: serde_json::Value, code: ErrorCode, message: &str) -> serde_json::Value {
    serde_json::to_value(ErrorEnvelope {
        id,
        error: ErrorPayload {
            code,
            message: message.to_string(),
        },
    })
    .expect("error should serialize")
}

fn read_message<R: Read>(reader: &mut R) -> Result<RequestEnvelope, KeystoneError> {
    let mut len_buf = [0_u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn write_message<W: Write>(writer: &mut W, value: &serde_json::Value) -> Result<(), KeystoneError> {
    let payload = serde_json::to_vec(value)?;
    let len = (payload.len() as u32).to_le_bytes();
    writer.write_all(&len)?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}
