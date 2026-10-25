# SPEC-176 Secret manager and custom providers

Provider API keys are stored through a tenant secret backend. The product runs with no Infisical deployment.

## Backends

- **Database (default).** AES-256-GCM with `EDGEQUAKE_SECRETS_KEY`, rows in `managed_secrets` (migration 222). No tenant config is required. When the key is unset the server still boots; saving a key returns `EDGEQUAKE_SECRETS_KEY is required to store API keys`.
- **Infisical (optional).** Chosen per tenant in Settings → Organization → Secrets. Universal Auth (`POST /api/v1/auth/universal-auth/login`) and the v4 secrets API. The client secret is envelope-encrypted in `secret_backend_configs`. The adapter is built only when that tenant's kind is `infisical`. A failed call is not written to the database.

Existing `provider_connections` ciphertext is still readable. New saves write `secret_ref` only.

## Custom providers

`GET /api/v1/connections/shapes` offers OpenAI-compatible (`openai_chat`), Anthropic-compatible (`anthropic_messages`) and Ollama. Other known aliases still save. Unknown shapes are rejected.

## Tests

- `cargo test -p edgequake-secrets`
- `cargo test -p edgequake-api --test spec176_secrets`
- `cd edgequake_webui && npx playwright test --project=mock-api e2e/spec176`
- Live (skipped without a stack): `e2e/spec176-live-secrets.spec.ts`
