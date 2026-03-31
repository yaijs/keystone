# Keystone

**Your local key host for secure connections in insecure browser terrain.**

## SECTION 1 — CANONICAL CONCEPT DOCUMENT

# Keystone

**Keystone is a local companion host for a browser extension that keeps long-lived LLM provider credentials out of extension storage and injects them only when forwarding a real provider request.**

## Problem

A browser extension can call LLM APIs, but it is a weak place to keep long-lived provider secrets. The usual alternatives are also bad: store keys in extension storage, or add a cloud relay or self-hosted backend. The first weakens local secret custody. The second adds infrastructure, cost, and an extra trust boundary.

Keystone exists to take the secret out of the extension without introducing a remote service. The user's own machine remains the trust and execution boundary.

## Product Statement

Keystone is a **local credential bridge and authenticated request tunnel** for one extension-first integration.

In v1, it is built for the plugin that needs it now. The first consumer is the plugin. The host may be extensible internally, but the product is not presented as a multi-extension platform, local database, or generic automation runtime.

The extension still owns UI, prompt construction, workflow logic, and request intent. Keystone owns provider credential custody, pairing, session issuance, and authenticated forwarding.

## Positioning

"Bridge" is the primary term.

"Proxy" is too broad and suggests a general-purpose local endpoint. "Gateway" suggests a larger policy and integration surface. "Bridge" is correct because the product crosses one specific boundary: browser extension execution on one side, OS-level secret custody and outbound provider access on the other. When more precision is needed, the full description is: **local credential bridge and authenticated request tunnel**.

## Architecture

Recommended target architecture:

- Native Messaging is the control plane and trust root.
- Localhost HTTP on `127.0.0.1` is the data plane.
- Keystone stores provider credentials in the OS credential store.
- The extension obtains a short-lived session over Native Messaging.
- The extension sends the actual LLM request to the returned localhost URL with the session token.
- Keystone validates the session, fetches the provider secret, injects provider auth, forwards the HTTPS request, and returns or streams the provider response.

This split is the right v1 target because it preserves a real trust bootstrap while keeping request transport simple and streaming-friendly.

POC path:

- Start with Native Messaging only if that is the fastest way to prove the plugin integration.
- Add the localhost HTTP data plane as soon as streaming, payload size, or protocol cleanliness makes Native Messaging-only awkward.

The localhost layer is transport only. It is not the primary identity mechanism.

## Trust and Security Model

Keystone improves the local boundary. It does not claim to secure a compromised machine.

What it improves:

- Long-lived provider keys are not stored in extension storage.
- Provider secrets are not exposed to normal extension UI code.
- Requests are tied to a paired extension identity, not a claimed header.
- Provider credentials are only used against explicitly allowed upstream URLs.

What it does not claim:

- It does not protect against malware or a fully compromised user session.
- It does not turn localhost into a trusted boundary by itself.
- It does not make arbitrary browser extensions safe to trust.

The trust chain starts with the Chrome Native Messaging host manifest, which limits `allowed_origins` to the intended extension ID, and continues with the runtime extension identity available through Native Messaging. `X-Consumer-ID`-style headers are not a trust model and are rejected as a primary identity mechanism.

### Threat Model

Assets protected:

- long-lived provider API keys
- request and response content while Keystone forwards it locally

Trusted components:

- the Keystone host process
- the OS credential store
- the paired extension identified through Native Messaging

Untrusted components:

- other browser extensions
- other local processes
- the localhost HTTP transport by itself

Out of scope:

- malware or an attacker with enough access to read Keystone process memory
- a fully compromised user session or operating system

## Pairing and Session Model

### Pairing

1. The plugin opens a Native Messaging connection.
2. Keystone reads the caller's extension identity from the Native Messaging context.
3. If the extension is unknown, Keystone shows a clear local approval prompt naming the plugin and its requested provider access.
4. Keystone stores a local trust record with extension ID, display name, first seen time, last used time, and allowed providers.
5. Future control and session requests are accepted only for paired extensions.

In the plugin-first v1, there is exactly one supported paired extension. The storage model can support more later, but the product and UX do not.

The approval path must be explicit in the implementation. The minimum acceptable flow is a small local window or system-level prompt that lets the user allow, deny, or cancel pairing. "Ask the user" is not enough as a hand-wave.

### Session Issuance

1. The paired plugin calls `llm.open_session` over Native Messaging for a named provider.
2. Keystone verifies that the extension is paired, that the provider is allowed, and that a credential exists.
3. Keystone returns a localhost base URL, a short-lived session token, expiry metadata, and allowed operations.
4. The plugin uses that token as a Bearer token on localhost HTTP requests.
5. Sessions are in-memory, short-lived, revocable, and cleared on restart.

If Keystone restarts, the extension must re-open a session over Native Messaging. No persistent token identity is assumed across restarts.

## API Direction

### Native Messaging

Use Native Messaging for:

- `bridge.hello`
- `bridge.pair`
- `bridge.status`
- `bridge.open_settings`
- `vault.list_providers`
- `vault.set_secret`
- `vault.delete_secret`
- `llm.open_session`

Native Messaging is not the place for the full streamed provider payload in the target architecture.

### Local HTTP

Keep the localhost API narrow in v1:

- `POST /v1/chat/completions`
- `GET /health`

Bind to `127.0.0.1` only. Use a random port. The extension learns the active port from the Native Messaging session response.

## Provider Model

V1 should support only the providers the plugin actually needs first. Do not broaden provider support for marketing symmetry.

Rules:

- Each provider has a named configuration and an explicit upstream base URL.
- Credentials are never attached to arbitrary URLs.
- Custom providers must use HTTPS and require explicit user confirmation.
- Keystone strips the session Bearer token before forwarding and injects only the provider auth required by the selected upstream.
- The plugin sends provider-native request bodies. Keystone does not normalize or translate between incompatible provider APIs in v1.
- The plugin does not choose arbitrary upstream URLs at request time. Provider and route selection remain bound to server-side configuration and session scope.

## MVP Scope

V1 is intentionally small:

- Native Messaging host in Rust
- OS credential store integration
- Pairing for the first plugin
- Short-lived in-memory sessions
- Localhost HTTP forwarding on a random loopback port
- `POST /v1/chat/completions` passthrough for the plugin's first required endpoint
- Streaming passthrough if the plugin needs streaming
- URL allowlist validation per provider
- Minimal diagnostics: configured providers, pairing state, recent request log

The first shipping milestone does not need a dedicated desktop UI beyond what is necessary for pairing and basic configuration. A full settings shell or tray app can follow after the host protocol and plugin flow are stable.

## Non-Goals

These are out of scope for v1 and should not appear in implementation planning:

- Generic local storage for plugin feature data
- Notes, bookmarks, feeds, vector search, or database hosting
- Plugin runtime or plugin marketplace
- Multi-user model or sync
- Cloud relay, hosted backend, or remote control plane
- Consumer isolation based on self-claimed headers
- Pause/resume state machines
- Broad multi-extension management UX

## Roadmap Direction

The roadmap should stay adjacent to the core mission:

- Better provider coverage only when driven by the plugin
- Cleaner diagnostics and recovery UX
- Better pairing and installer flow
- Optional later support for additional trusted extensions if a real second consumer exists
- Optional local model adapters later if they fit the same credential bridge pattern

Healthy extensibility means reusable internal modules. Unhealthy extensibility means promising a local platform before the bridge proves itself in real plugin use.

## Final Definition

Keystone is a local credential bridge for a browser extension that needs to call LLM provider APIs without keeping long-lived provider credentials in browser storage.

It pairs the extension through Native Messaging, issues short-lived local sessions, injects provider credentials only at forward time, and returns the real provider response over a narrow localhost tunnel.

Local-first. No cloud relay. Plugin-first scope.

## SECTION 2 — DESIGN DECISIONS AND REJECTED ALTERNATIVES

### Decisions

- **Bridge over proxy/gateway.** "Bridge" best matches the product boundary and avoids implying a general-purpose local proxy.
- **Native Messaging for trust, localhost HTTP for data.** This is the recommended architecture because it separates identity bootstrap from request transport cleanly.
- **Plugin-first v1.** The first version is scoped to one real extension integration. Data structures can support more later, but the product should not.
- **Random loopback port with session discovery via Native Messaging.** This avoids fixed-port assumptions and makes restart recovery explicit.
- **OS credential store for provider secrets.** Long-lived secrets stay outside extension storage.
- **Short-lived in-memory sessions.** Restart clears them. Re-bootstrap is explicit.
- **Minimal UI.** Useful, but secondary to the bridge protocol and first integration.
- **Provider-native passthrough.** Keystone forwards provider-native request shapes and injects auth; it does not become a schema translation layer in v1.

### Rejected

- **`X-Consumer-ID` as the trust anchor.** Rejected because any local caller can forge it.
- **Generic local platform framing.** Rejected because it dilutes the product and invites premature database and plugin-runtime work.
- **Broad multi-extension positioning in v1.** Rejected because there is only one real consumer right now.
- **Pause/resume lifecycle features.** Rejected because they add state and support complexity without solving the actual product problem.
- **Fixed localhost port.** Rejected because it is unnecessary, conflict-prone, and encourages treating HTTP as a trust root.
- **Large v1 desktop shell requirements.** Rejected because they delay the plugin integration that validates the concept.
- **Provider schema normalization in the bridge.** Rejected because it broadens the host into a provider abstraction layer before the first plugin flow is proven.

### Useful Pressure-Test Points

- The critique was right that the transport split needs explicit answers for port discovery, restart behavior, and session lifetime.
- The critique was right that shared generic infrastructure is a risk before the first plugin proves the flow.
- The critique was right that pairing UX, installer flow, and host recovery behavior need to be described as product work, not left as implied implementation details.
- The earlier concept overreached by adding generic storage, consumer namespaces, pause/resume, and platform language.

### Intentionally Open

- Whether the first release ships with manual updates only or includes an updater. This affects packaging and code-signing work, not the core bridge design.
- How development, beta, and store extension IDs are handled in the Native Messaging manifest and pairing records without creating an unsafe trust shortcut.

## SECTION 3 — IMPLEMENTATION APPENDIX FOR CODEX

## Recommended v1 Architecture

- Rust host process
- Native Messaging control channel
- Loopback HTTP server on `127.0.0.1` with a random port
- OS credential store integration
- Local trust record store for the paired plugin
- In-memory session store with TTL
- Narrow provider forwarding layer with explicit URL allowlists

If the fastest path to a working plugin integration is Native Messaging-only, build that first. Treat it as the proving step, not the final architecture.

## Recommended Transport Split

- **Control plane:** Native Messaging
- **Data plane:** localhost HTTP

Control plane responsibilities:

- host discovery
- pairing
- provider management
- session issuance
- settings/status
- install-time trust binding through the Native Messaging host manifest

Data plane responsibilities:

- authenticated LLM request forwarding
- streaming passthrough
- narrow health endpoint

## Pairing Flow

1. Plugin sends `bridge.hello`.
2. Host checks for an existing trust record by extension ID.
3. If absent, plugin sends `bridge.pair`.
4. Host presents a local approval UI that names the plugin and requested provider scope.
5. Host stores the trust record with allowed providers.
6. Later starts skip approval and return paired status immediately.

In v1, reject any second unrelated extension instead of pretending multi-extension support exists.

## Session Issuance Flow

1. Plugin sends `llm.open_session` with a provider ID.
2. Host verifies pairing, provider permission, and secret presence.
3. Host generates a strong random token and expiry.
4. Host stores `{token, extension_id, provider_id, allowed_ops, expires_at}` in memory.
5. Host returns `{base_url, token, expires_at}`.
6. Plugin sends localhost requests with `Authorization: Bearer <token>`.
7. Host validates token and provider scope before forwarding.

Tokens are not persisted. Restart invalidates all of them.

## Localhost API Direction

Required:

- `POST /v1/chat/completions`
- `GET /health`

Rules:

- bind to `127.0.0.1` only
- require Bearer session auth on all endpoints
- reject missing, expired, or unknown tokens with `401`
- do not forward the session token upstream
- inject provider auth only after provider and URL validation succeeds

## Native Messaging API Direction

Required methods:

- `bridge.hello`
- `bridge.pair`
- `bridge.status`
- `bridge.open_settings`
- `vault.list_providers`
- `vault.set_secret`
- `vault.delete_secret`
- `llm.open_session`

Suggested response shape:

- stable request `id`
- `result` or `error`
- explicit machine-readable error codes for unpaired, unknown provider, missing secret, and rejected pairing

Default contract:

- `bridge.hello` returns host version, extension ID seen, pairing state, and supported methods, but no session material
- `bridge.pair` is idempotent
- `llm.open_session` takes exactly one provider and one operation and returns one scoped short-lived session
- `vault.set_secret` remains available over Native Messaging in v1, but only for a paired extension and with explicit local confirmation before storage

The installer must write a Chrome Native Messaging host manifest whose `allowed_origins` contains only the intended extension ID for the first plugin.

## Provider Validation Rules

- Provider IDs are explicit and finite.
- Each provider maps to one allowed upstream base URL set.
- Custom providers require explicit user configuration and HTTPS.
- Credentials must never be attached to a request whose upstream URL is outside the configured allowlist.
- The plugin does not choose arbitrary upstream targets once a provider session is issued.
- The bridge forwards provider-native payloads and response bodies; it does not translate OpenAI, Anthropic, Gemini, and other schemas into one internal format in v1.

## Restart and Reconnect Behavior

- Host restart selects a new random port.
- All sessions are cleared.
- Pairing records survive.
- Plugin treats `401` or connection failure as a signal to re-run `bridge.hello` and `llm.open_session`.
- No silent session refresh from the HTTP layer.

## Failure Modes the Implementation Must Handle

- host not installed or not running
- host manifest missing or `allowed_origins` not matching the plugin ID
- pairing rejected by the user
- unknown extension ID
- missing provider credential
- expired session
- provider unreachable
- upstream provider error body passthrough
- port bind failure
- host restart during an active plugin session
- provider key deleted after session issuance
- unsigned or improperly signed host binary blocked by the OS
- stale session replaced by a newer session for the same provider

Each failure must produce a clear machine-readable error and a user-facing recovery path in the plugin.

## Minimal Project Structure Recommendation

```text
keystone/
├── host/
│   ├── src/
│   │   ├── main.rs
│   │   ├── native_messaging.rs
│   │   ├── pairing.rs
│   │   ├── vault.rs
│   │   ├── session.rs
│   │   ├── http_server.rs
│   │   ├── provider.rs
│   │   ├── forwarder.rs
│   │   ├── config.rs
│   │   └── db.rs
│   └── Cargo.toml
├── protocol/
│   └── PROTOCOL.md
└── installer/
```

Add a small UI module only after the host protocol and plugin integration are working end to end.

## Build This First

1. Native Messaging host skeleton with `bridge.hello`.
2. OS credential store wrapper for one provider.
3. Pairing flow for the first plugin and persisted trust record.
4. `llm.open_session` with in-memory token store and TTL.
5. Loopback HTTP server with `POST /v1/chat/completions`.
6. Provider auth injection and URL allowlist enforcement.
7. End-to-end plugin request against a real provider.
8. Native Messaging host manifest generation and installer flow.
9. Minimal diagnostics surface and log redaction rules.

## Open Questions

- What is the minimum acceptable signed-installer and update story for Windows and macOS?
