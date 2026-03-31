# Keystone Dev Harness

This file documents the fastest local smoke path without Chrome.

## 1. Check runtime flavor and extension identity

```bash
KEYSTONE_FLAVOR=dev KEYSTONE_EXTENSION_ID_OVERRIDE=yourdevextensionid \
  cargo run --bin keystone-dev -- runtime-info
```

## 2. Generate Native Messaging request payloads

```bash
cargo run --bin keystone-dev -- hello
cargo run --bin keystone-dev -- pair openai
cargo run --bin keystone-dev -- set-secret openai sk-...
cargo run --bin keystone-dev -- open-session openai
```

## 3. Run the host itself

```bash
KEYSTONE_FLAVOR=dev KEYSTONE_EXTENSION_ID_OVERRIDE=yourdevextensionid cargo run --bin keystone
```

The host expects real Chrome Native Messaging framing on `stdin`, so the helper binary is currently for payload generation and inspection rather than a full framed transport client.

## 4. Automated local smoke test

```bash
cargo run --bin keystone-dev -- smoke
cargo run --bin keystone-dev -- smoke-persist
```

This smoke path:

- starts `keystone`
- forces `KEYSTONE_FLAVOR=dev`
- forces `KEYSTONE_IN_MEMORY_VAULT=1`
- issues framed Native Messaging requests
- calls authenticated `/health`

It avoids persistent secret writes and avoids upstream API calls.

`smoke-persist` adds one more check:

- restart Keystone after pairing
- run `bridge.hello` again
- verify the persisted pairing state is loaded on startup

## 5. Current limitation

The host is real enough now for:

- runtime flavor separation
- keyring-backed secret storage
- session issuance
- authenticated HTTP request handling
- OpenAI-compatible forwarding path

But the harness is still intentionally minimal:

- it does not yet drive `/v1/chat/completions` against a real upstream
- it does not yet simulate Chrome launch or installer registration
- it does not yet verify pairing persistence across runs

Those are the next additions if you want a fuller integration harness.
