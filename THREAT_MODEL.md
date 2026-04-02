# Keystone Threat Model

This document defines the current security model for Keystone as it exists today and the minimum assumptions clients should make when integrating with it.

Keystone is a browser-extension-first local credential proxy with managed secret storage.

Its job is to improve provider-secret handling for supported local clients, especially browser extensions.

It is not a complete local security boundary and should not be described that way.

## 1. Security Goal

Keystone exists to reduce exposure from storing long-lived provider API keys directly in browser-extension storage.

It does that by:
- storing provider secrets in the OS keyring instead of extension storage
- keeping long-lived provider credentials outside the extension runtime
- issuing short-lived local sessions for proxied requests
- injecting provider credentials only during forwarding

This is an improvement in secret handling.
It is not a guarantee against local compromise or client misuse.

## 2. What Keystone Protects Against

Keystone is designed to improve the default browser-extension model in these areas:

- long-lived provider keys do not need to live in extension storage
- normal extension UI/config flows do not need to directly manage the raw provider secret
- provider credentials are attached only during proxied requests
- provider access can be scoped through pairing and session issuance

In practical terms, Keystone is meant to be safer than:
- storing provider keys directly in `chrome.storage.local`
- keeping raw API keys in extension-managed config as the primary model

## 3. What Keystone Does Not Protect Against

Keystone does not claim to protect against:

- a fully compromised local user account
- local malware with enough access to inspect Keystone process memory
- a compromised operating system
- an authorized but malicious client misusing the access it has already been granted
- same-user isolation between multiple local clients unless that separation is explicitly enforced

Keystone improves secret handling.
It does not eliminate trust risk.

## 4. Main Trust Boundaries

Current trust boundaries are:

- the OS keyring for secrets at rest
- the Native Messaging trust path for browser-extension identity
- Keystone's own local process state
- short-lived session validation state inside Keystone

These are not trust boundaries by themselves:

- localhost reachability
- random local ports
- hidden admin URLs
- self-claimed headers from arbitrary local callers

## 5. Assets Being Protected

Primary protected assets:

- long-lived provider API keys
- provider association and configured secret state
- local trust records
- active session tokens

Secondary sensitive data:

- request/response bodies while Keystone forwards them
- local admin status information
- install paths and support-file locations

## 6. Trusted Components

For the current intended use case, Keystone assumes trust in:

- the Keystone process itself
- the OS keyring implementation
- the browser's Native Messaging launch model
- the installed host manifest and its `allowed_origins` restriction

This trust is conditional and local.
It should not be described as stronger than it really is.

## 7. Untrusted Components

Keystone treats the following as untrusted by default:

- other local processes
- other browser extensions
- localhost traffic by itself
- arbitrary local callers that are not authenticated through the intended control path
- upstream providers beyond their specific API role

## 8. Browser-Extension Model

The current primary client class is browser extensions.

Keystone relies on:
- Native Messaging manifest installation
- browser-provided extension origin
- trust records scoped to extension ID and flavor

Important caveat:

An extension ID is useful for trust binding.
It is not proof that the extension code itself cannot be compromised or abused.

If a paired extension becomes malicious or compromised, Keystone does not inherently stop that extension from misusing already granted access.

## 9. Localhost Model

Localhost is used as a transport mechanism for the proxied request data plane.

Rules:
- bind to `127.0.0.1` only
- use short-lived bearer tokens
- keep sessions scoped and short-lived

Important:

Localhost is not trusted just because it is localhost.

Any local control or forwarding path must still be authenticated and scoped.

## 10. Session Model

The intended session security posture is:

- short-lived tokens
- server-side validation state
- session scope bound to:
  - client identity
  - flavor
  - provider
  - operation
- no persistence across restart unless explicitly redesigned later

Current product intent:
- sessions are ephemeral
- restart should invalidate them

If a session token is stolen while valid, it may be replayed until it expires or is revoked.
That is why short TTL and strict claim binding matter.

## 11. Secrets In Memory

Keystone uses the OS keyring to improve secret storage at rest.

That does not mean secrets never enter memory.

During active use:
- Keystone may read the provider secret from the keyring
- Keystone may hold it in process memory long enough to inject it into the forwarded request

Therefore:
- keyring use protects secrets at rest
- it does not protect against a process-memory compromise

## 12. Workspaces And Isolation

Current Keystone releases should not claim that workspaces provide hard isolation.

If workspaces exist or are added later, they should currently be understood as:
- organizational namespace
- secret grouping
- trust-policy scope

They should not be marketed as:
- hard same-user isolation
- multi-tenant security boundary

## 13. Admin Plane Assumptions

The admin surface is security-sensitive.

Current required principles:
- localhost-only binding
- explicit authentication for mutating operations
- no reliance on hidden URLs as the main protection
- clear separation between read-only diagnostics and write operations

Until admin authentication is fully solid:
- admin write capabilities should be treated conservatively
- CLI is preferred for sensitive recovery/mutation flows

## 14. Upstream Provider Restrictions

Keystone must not become a generic credential oracle.

That means:
- each provider must map to explicit allowed upstream targets
- provider identity must be bound into session scope
- clients must not be able to redirect one provider grant to arbitrary unrelated destinations

## 15. Logging Rules

Security-relevant logging should be small and disciplined.

Keystone should log:
- trust changes
- secret mutation events
- session lifecycle events
- install/uninstall actions
- admin-auth failures

Keystone should not log:
- raw provider secrets
- raw Authorization headers
- full request bodies by default

## 16. Practical Security Summary

The most honest summary of Keystone is:

- it is safer than storing provider secrets directly in browser-extension storage
- it keeps long-lived provider secrets in the OS keyring instead of the extension
- it still requires trust in the local machine, the Keystone process, and the authorized client
- it reduces exposure, but it does not eliminate local trust risk

## 17. Product Language That Should Be Avoided

Do not describe Keystone as:

- a key guard
- a hard security boundary
- real isolation
- a general secure local runtime
- safe just because it uses localhost

Prefer language like:

- local credential proxy
- managed secret storage
- browser-extension companion
- authenticated local request tunnel
- improved local secret handling

## 18. Open Security Questions

These still need explicit product decisions:

- exact admin-token lifecycle and storage model
- whether and how secrets are cached in memory beyond active forwarding
- how strict session revocation should be
- whether workspaces remain organizational only or get stronger enforcement
- how local trust approval should work on each supported OS
