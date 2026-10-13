# Model connections

Tuck uses **Apple Intelligence** on your Mac by default. In **Settings → AI model → Other** you can
connect another model instead:

| Provider | Endpoint | Notes |
| --- | --- | --- |
| **Ollama** | `http://127.0.0.1:11434` by default | Any installed model; **Find models** lists them |
| **OpenAI-compatible API** | the service's base URL, usually ending in `/v1` | Hosted services and local servers such as LM Studio |
| **Anthropic** | `https://api.anthropic.com` | The native Messages API |

**Test connection** sends a short synthetic greeting, never a document. **Save** applies the
settings and stores a typed API key in macOS Keychain; leaving the key empty keeps the saved one.
Keys are stored per connection type and exact endpoint, so changing the endpoint never sends the old
endpoint's key.

## Privacy

Apple Intelligence processes everything on this Mac. A custom connection receives the extracted text
of each document (the first 5,000 characters), its file clues and your folder rules. Ollama can run
locally but can also forward to a remote model, so Tuck does not promise on-device processing for
custom connections. The provider's own terms, retention and charges apply.

## Safety limits

The helper accepts HTTPS endpoints, and plain HTTP only on the loopback address. It rejects URLs with
credentials, queries or fragments; disables redirects, cookies, proxies and caching; caps responses
at 1 MB and requests at 85 seconds; and rejects truncated, malformed or refused replies and tool
calls. API keys are never written to `.tuck/settings.json`, sent back to the interface or included in
error messages. No model can call tools or touch files.

Every model gets the same requests and the same validation. Settings belong to the chosen folder,
and changing the model stops automatic filing and discards decisions made with the previous model.

## Implementation

- `src-tauri/swift/TuckConnections.swift`: the three protocol adapters and Keychain access.
- `src-tauri/swift/TuckModelChoice.swift`: routes each request to Apple or the connection.
- `src-tauri/src/provider.rs`: the request contract shared by all models.
- `npm run test:contracts` runs `evals/connection-contract.py`, which starts local mock servers and
  checks all three protocols: arbitrary model IDs, model listing, authentication errors, redirects,
  truncation, card cleaning and folder choice. These are protocol tests, not accuracy measurements.

Protocol references: [Ollama chat](https://docs.ollama.com/api/chat),
[OpenAI Chat Completions](https://platform.openai.com/docs/api-reference/chat/create),
[Anthropic Messages](https://docs.anthropic.com/en/api/messages).
