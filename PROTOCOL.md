# Keystone Protocol v1

This document defines the v1 protocol for Keystone as described in [CONCEPT.keystone.latest.md](./CONCEPT.keystone.latest.md).

The design is intentionally narrow:

- one paired plugin first
- Native Messaging as the trust root
- localhost HTTP as the request data plane
- provider-native passthrough, not schema normalization

## 1. Scope

Keystone v1 supports:

- host discovery and version negotiation
- plugin pairing
- provider listing and secret management
- short-lived session issuance
- authenticated localhost forwarding for LLM provider requests

Keystone v1 does not support:

- generic local storage
- multi-extension product UX
- session persistence across restarts
- provider schema translation

## 2. Transport Model

### 2.1 Native Messaging

Native Messaging is the control plane.

It is used for:

- `bridge.hello`
- `bridge.pair`
- `bridge.status`
- `bridge.open_settings`
- `vault.list_providers`
- `vault.set_secret`
- `vault.delete_secret`
- `llm.open_session`

Trust model:

- Chrome launches the host only if the Native Messaging manifest is installed correctly.
- The manifest `allowed_origins` must contain only the intended extension origin in v1.
- Keystone treats the Native Messaging caller identity as the only trust root.

### 2.2 Localhost HTTP

Localhost HTTP is the data plane.

It is used for:

- `POST /v1/chat/completions`
- `GET /health`

Rules:

- bind to `127.0.0.1` only
- select a random available port on startup
- require `Authorization: Bearer <session_token>` on all endpoints
- never treat localhost reachability alone as trust

## 3. Native Messaging Wire Format

Chrome Native Messaging uses:

- 4-byte unsigned little-endian length prefix
- UTF-8 JSON payload

Each request must be a JSON object:

```json
{
  "id": "c1f4f1cc-8158-4c6d-8c73-1c2acfd64318",
  "method": "bridge.hello",
  "params": {
    "protocol_version": "1.0"
  }
}
```

Each successful response must be:

```json
{
  "id": "c1f4f1cc-8158-4c6d-8c73-1c2acfd64318",
  "result": {}
}
```

Each error response must be:

```json
{
  "id": "c1f4f1cc-8158-4c6d-8c73-1c2acfd64318",
  "error": {
    "code": "EXTENSION_NOT_PAIRED",
    "message": "The calling extension is not paired with this Keystone instance."
  }
}
```

Rules:

- `id` is plugin-generated and echoed unchanged
- exactly one of `result` or `error` must be present
- unknown methods return `METHOD_NOT_FOUND`
- malformed payloads return `INVALID_REQUEST`

## 4. Native Messaging Methods

### 4.1 `bridge.hello`

Purpose:

- establish basic connectivity
- return protocol compatibility
- report pairing state

Request:

```json
{
  "id": "1",
  "method": "bridge.hello",
  "params": {
    "protocol_version": "1.0",
    "extension_name": "Y/TXT"
  }
}
```

Success result:

```json
{
  "protocol_version": "1.0",
  "host_version": "0.1.0",
  "extension_id_seen": "abcdefghijklmnoabcdefhijklmnoab",
  "pairing_status": "paired",
  "supported_methods": [
    "bridge.hello",
    "bridge.pair",
    "bridge.status",
    "vault.list_providers",
    "vault.set_secret",
    "vault.delete_secret",
    "llm.open_session"
  ],
  "capabilities": {
    "http_data_plane": true,
    "responses_api": false
  }
}
```

If the extension is not paired, `pairing_status` must be `unpaired`.

`bridge.hello` must not create or refresh a session.

### 4.2 `bridge.pair`

Purpose:

- request explicit local approval for the calling extension

Request:

```json
{
  "id": "2",
  "method": "bridge.pair",
  "params": {
    "extension_name": "Y/TXT",
    "requested_providers": ["openai"]
  }
}
```

Success result:

```json
{
  "pairing_status": "paired",
  "allowed_providers": ["openai"]
}
```

Behavior:

- host must present a local approval prompt
- user may allow, deny, or dismiss
- dismiss should not create a trust record
- if the calling extension is already paired, this method should be idempotent and return the existing allowed provider scope

### 4.3 `bridge.status`

Purpose:

- expose minimal diagnostics to the plugin or local UI

Success result:

```json
{
  "host_version": "0.1.0",
  "uptime_seconds": 124,
  "pairing_status": "paired",
  "active_sessions": 1,
  "providers": [
    { "id": "openai", "configured": true }
  ]
}
```

### 4.4 `bridge.open_settings`

Purpose:

- open a minimal local settings or pairing window if present

Success result:

```json
{
  "ok": true
}
```

If the host is headless in the first build, this may open a minimal config surface or return `NOT_SUPPORTED`.

### 4.5 `vault.list_providers`

Success result:

```json
{
  "providers": [
    {
      "id": "openai",
      "display_name": "OpenAI",
      "configured": true
    }
  ]
}
```

### 4.6 `vault.set_secret`

Purpose:

- store or replace a provider secret

Request:

```json
{
  "id": "3",
  "method": "vault.set_secret",
  "params": {
    "provider": "openai",
    "secret": "sk-..."
  }
}
```

Success result:

```json
{
  "ok": true
}
```

Rules:

- secret must be written to the OS credential store
- secret must never be logged
- host must require the extension to be paired before accepting this method
- host should require an explicit local confirmation before first storage or replacement in v1
- if the provider is custom, upstream URL validation must already be configured or supplied and confirmed

### 4.7 `vault.delete_secret`

Request:

```json
{
  "id": "4",
  "method": "vault.delete_secret",
  "params": {
    "provider": "openai"
  }
}
```

Success result:

```json
{
  "ok": true
}
```

### 4.8 `llm.open_session`

Purpose:

- create a short-lived localhost session for one provider

Request:

```json
{
  "id": "5",
  "method": "llm.open_session",
  "params": {
    "provider_id": "openai",
    "operation": "chat.completions"
  }
}
```

Success result:

```json
{
  "base_url": "http://127.0.0.1:54321",
  "session_id": "sess_01hq...",
  "token": "9d72c1f2...",
  "expires_at": "2026-03-19T12:00:00Z",
  "provider_id": "openai",
  "allowed_operation": "chat.completions"
}
```

Rules:

- token must be cryptographically random
- token must be stored in memory only
- token must be scoped to the paired extension, one provider, and one operation
- host restart invalidates all tokens
- default TTL is 5 minutes
- v1 default: one active session per provider per extension; opening a new session for the same provider invalidates the previous one

## 5. HTTP API

## 5.1 Authentication

Every HTTP request must include:

```http
Authorization: Bearer <session_token>
```

If missing, invalid, expired, or scoped to the wrong provider, the host must return `401` or `403` as appropriate.

## 5.2 `GET /health`

Purpose:

- lightweight check that the HTTP listener is alive

Success response:

```json
{
  "status": "ok",
  "host_version": "0.1.0"
}
```

This endpoint still requires a valid session token in v1.

## 5.3 `POST /v1/chat/completions`

Purpose:

- forward a provider-native chat request after auth injection

Rules:

- request body is accepted as provided by the plugin
- Keystone validates session and provider scope
- Keystone resolves the configured provider upstream URL
- Keystone fetches the provider secret from the OS store
- Keystone strips the session Bearer token
- Keystone injects the provider auth header
- Keystone forwards the body without schema translation

Success:

- return provider response body as-is when practical
- support streaming passthrough if the provider response is streamed

### 5.4 `POST /v1/responses`

Optional in v1.

Only expose this route if the plugin's first real integration needs it.

Behavior matches `/v1/chat/completions`:

- session validation
- provider auth injection
- provider-native passthrough

The plugin must not supply arbitrary upstream URLs through these endpoints. Route and upstream selection are bound server-side from the issued session plus provider configuration.

## 6. Provider Configuration Rules

Provider definitions are explicit.

Each provider entry must define:

- provider ID
- display name
- upstream base URL
- auth injection method

Example built-ins:

- `openai` -> `Authorization: Bearer {secret}`
- `deepseek` -> `Authorization: Bearer {secret}`
- `anthropic` -> `x-api-key: {secret}` plus any fixed required headers

Rules:

- credentials must never be attached to arbitrary URLs
- custom providers must use HTTPS
- custom providers must require explicit user confirmation
- the plugin must not choose an arbitrary upstream host once a session is issued

## 7. Error Model

Native Messaging error codes:

- `INVALID_REQUEST`
- `METHOD_NOT_FOUND`
- `EXTENSION_NOT_PAIRED`
- `PAIRING_REJECTED`
- `PAIRING_CANCELLED`
- `PROVIDER_UNKNOWN`
- `PROVIDER_NOT_ALLOWED`
- `PROVIDER_NOT_CONFIGURED`
- `SESSION_LIMIT_REACHED`
- `NOT_SUPPORTED`
- `INTERNAL_ERROR`

Extension-facing install and bootstrap failure classes:

- `HOST_NOT_FOUND`
- `MANIFEST_INVALID`
- `ORIGIN_NOT_ALLOWED`

HTTP status guidance:

- `401 Unauthorized` -> missing, invalid, or expired session token
- `403 Forbidden` -> session exists but requested provider or route is not allowed
- `502 Bad Gateway` -> upstream provider error or unreachable upstream
- `503 Service Unavailable` -> host temporarily unable to serve request

Error bodies must be machine-readable and must avoid leaking secrets.

## 8. Restart and Recovery

On host restart:

- select a new random port
- clear all sessions
- preserve pairing records
- require the plugin to call `bridge.hello` and `llm.open_session` again

The plugin must treat:

- Native Messaging disconnect
- HTTP connection failure
- HTTP `401`

as signals to re-bootstrap the session.

## 9. Logging Rules

Safe to log:

- timestamp
- extension ID
- provider ID
- route
- success or failure status
- upstream status code

Never log:

- provider secrets
- session tokens
- raw request or response bodies by default
- full authorization headers

## 10. First Implementation Defaults

Recommended defaults:

- one paired extension only
- one provider first: `openai`
- session TTL: 5 minutes
- one active session per provider per extension is acceptable for v1
- headless or minimal local UI is acceptable in the first shipping build

## 11. Installer and Manifest Requirements

The first shipping build must treat installation as part of the protocol surface.

Requirements:

- fixed host name, for example `com.ytxt.keystone`
- exact extension ID in manifest `allowed_origins`
- stable installed binary path referenced by the manifest
- per-OS manifest placement handled by the installer
- post-install smoke test that verifies `bridge.hello`

The design must also account for different extension IDs between development, beta, and store builds. That mapping cannot be left implicit.

## 12. Review Excerpts

The following points should be re-reviewed if implementation friction appears:

- whether `/v1/responses` belongs in v1
- whether the first release should expose any unauthenticated health check
- whether `vault.set_secret` should remain available over Native Messaging in the first shipping build or move behind a local-only UI
