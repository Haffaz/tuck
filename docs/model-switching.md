# Model connections

Open **Settings → AI connection**. Apple Intelligence remains the default. Choose **Add connection** for an arbitrary text model served by:

- **Ollama**: default base URL `http://127.0.0.1:11434`; enter an installed model ID, or use **Find models**.
- **OpenAI-compatible API**: enter the service’s API base URL (including `/v1` where required), model ID and API key. This covers compatible hosted services and local servers such as LM Studio; it does not imply support for every provider’s proprietary protocol.
- **Anthropic**: native Messages API, with a free model ID and API key.

**Test connection** sends a synthetic greeting without documents. **Find models** queries the model catalog. Neither saves your changes. **Save settings** applies the draft and stores a supplied key in macOS Keychain. Leaving the key empty keeps the saved key. Keys are scoped to the connection type and exact normalized API endpoint; changing an endpoint never forwards the old endpoint’s key. Changing a draft endpoint clears a newly typed key.

Settings belong to the chosen document folder. Model changes stop organizing/watching and invalidate older decisions. Start Tuck again explicitly. Legacy Gemma selections still load and run; there is no automatic provider fallback.

## Privacy and limits

Apple Intelligence processes text on this Mac. Other connections receive extracted document text and filing instructions. A local Ollama server can itself host or forward to a remote model, so the UI does not promise on-device processing for custom connections. Provider terms, retention and charges apply.

The native helper requires macOS 26+. It accepts HTTPS endpoints and HTTP loopback endpoints only, rejects URL credentials/query/fragment, disables HTTP redirects, cookies, proxies and response caching, limits responses to 1 MB and requests to 85 seconds. It rejects truncated, malformed, refused and tool-call responses. API keys are never persisted in `.tuck/settings.json`, never returned to the webview, and never included in diagnostic errors. Models cannot move files or call filesystem tools.

Technical limits are automatic. The evidence reader accepts complete short documents up to 5,000 characters; incomplete, longer or unclear documents stay for review. Grouping batches preserve exact document IDs and bounded source evidence. Strict response validation and independent filing verification apply to custom providers as well as Apple.

## Categories

**Settings → Folder nesting** sets the maximum depth of new folder suggestions: one level or up to two levels. Two levels allow `Work/Orion`. Fewer levels are always valid. The limit is included in the prompt and enforced when the response is validated; creation preferences cannot override it.

Optional **Folder creation preferences**, beside this control, describe how folders should be grouped. Details such as employers or projects must come from source evidence. Existing folder rules continue to describe what belongs in each saved folder and are inherited by children. **Document instructions** apply to filing and naming across folders.

Changing nesting from two to one affects new suggestions and leaves existing two-level rules usable. Review a proposal and select **Use these categories** before creating folders. Legacy specificity settings remain readable but are no longer used; older settings default to up to two levels. Saved destinations are capped at two levels. Older three-level settings load at two; deeper rule files and documents remain untouched, with a notice in Settings. They are not active destinations.

## Implementation and checks

`provider::configured` routes all production roles through the saved preferences. `TuckConnections.swift` handles protocol adapters and Keychain access. `TuckLocalTransport.swift` retains legacy local selections. Connection details, folder nesting and optional creation instructions participate in settings revisions; description caches are scoped by model configuration and helper hash.

- `cargo test --manifest-path src-tauri/Cargo.toml`: settings migration, revisions, URL constraints, filing behavior.
- `npm run test:ui`: explicit save/discard, category tree, nesting and API credentials.
- `python3 evals/connection-contract.py`: local synthetic servers test all three protocols, arbitrary IDs, discovery, auth errors, redirects, truncation and the card operations. These are protocol tests, not live provider accuracy claims.

Protocol references: [OpenAI Chat Completions](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create), [Ollama chat](https://docs.ollama.com/api/chat), [Anthropic Messages](https://platform.claude.com/docs/en/api/messages/create).
