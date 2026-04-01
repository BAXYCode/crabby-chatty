# crabby-specs

Single source of truth for WebSocket message contracts and AsyncAPI documentation.

## What it defines

### WebSocket message types

- **`CrabbyWsFromClient`** — Messages sent by clients (e.g. `UserMessage` with destination, contents, timestamp).
- **`CrabbyWsFromServer`** — Messages sent by the server (e.g. `ChatMessage` with a server-assigned `message_id`).
- **`Destination`** — Routing target: `Individual { id }` for DMs, `Group { id }` for group messages.

### AsyncAPI spec

The crate derives an [AsyncAPI](https://www.asyncapi.com/) specification from the message types, ensuring documentation stays in sync with the code.

## Binaries

- **`build_spec`** — Generates the AsyncAPI spec document.

## Design intent

By centralizing message schemas here, `crabby-chat` (and any future clients or services) share compile-time–checked contracts, preventing drift between producer and consumer.
