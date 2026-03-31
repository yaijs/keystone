use axum::http::{HeaderMap, HeaderValue};
use serde_json::Value;

use crate::app::AppState;
use crate::error::KeystoneError;
use crate::provider::{AuthScheme, ProviderApiStyle};

pub async fn forward_chat_completions(
    state: &AppState,
    provider_id: &str,
    payload: Value,
) -> Result<(u16, Value), KeystoneError> {
    forward_request(state, provider_id, ProviderApiStyle::ChatCompletions, payload).await
}

pub async fn forward_messages(
    state: &AppState,
    provider_id: &str,
    payload: Value,
) -> Result<(u16, Value), KeystoneError> {
    forward_request(state, provider_id, ProviderApiStyle::AnthropicMessages, payload).await
}

async fn forward_request(
    state: &AppState,
    provider_id: &str,
    expected_api_style: ProviderApiStyle,
    payload: Value,
) -> Result<(u16, Value), KeystoneError> {
    let provider = state
        .providers
        .get(provider_id)
        .ok_or_else(|| KeystoneError::Internal(format!("unknown provider: {provider_id}")))?;

    if provider.api_style != expected_api_style {
        return Err(KeystoneError::Internal(format!(
            "provider {provider_id} does not support this operation"
        )));
    }

    let secret = {
        let vault = state.vault.lock().await;
        vault
            .get_secret(provider_id)
            .ok_or_else(|| KeystoneError::Internal(format!("missing secret for provider: {provider_id}")))?
    };

    let mut headers = HeaderMap::new();
    let auth_value = match provider.auth_scheme {
        AuthScheme::Bearer => format!("Bearer {secret}"),
        AuthScheme::Raw => secret,
    };
    headers.insert(
        provider.auth_header,
        HeaderValue::from_str(&auth_value)
            .map_err(|err| KeystoneError::Internal(format!("invalid auth header: {err}")))?,
    );
    for (name, value) in provider.extra_headers {
        headers.insert(
            *name,
            HeaderValue::from_str(value).map_err(|err| {
                KeystoneError::Internal(format!("invalid static header {name}: {err}"))
            })?,
        );
    }

    let endpoint_path = match provider.api_style {
        ProviderApiStyle::ChatCompletions => "/v1/chat/completions",
        ProviderApiStyle::AnthropicMessages => "/v1/messages",
    };
    let response = state
        .http_client
        .post(format!("{}{}", provider.base_url, endpoint_path))
        .headers(headers)
        .json(&payload)
        .send()
        .await
        .map_err(|err| KeystoneError::Internal(format!("upstream request failed: {err}")))?;

    let status = response.status().as_u16();
    let body = response
        .json::<Value>()
        .await
        .map_err(|err| KeystoneError::Internal(format!("invalid upstream json response: {err}")))?;

    Ok((status, body))
}
