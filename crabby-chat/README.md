# crabby-chat

Real-time messaging service built on Axum WebSockets and the [kameo](https://docs.rs/kameo) actor framework.

## Architecture

The service exposes a single WebSocket endpoint (`:6969/ws`) and uses an actor-based design for concurrency:

- **`EngineActor`** — Central hub that tracks all connected users (`HashMap<Uuid, Recipient>`) and routes messages between them (direct or group).
- **`IncomingMessageActor`** — Reads raw WebSocket frames, decodes them into domain types via the `Decode` trait, and forwards them to the engine.
- **`OutgoingMessageActor`** — Encodes domain messages into binary WebSocket frames (via `ServerToTransport` / `Encode`) and writes them to the client sink.

### Message flow

```
Client WS frame
  -> IncomingMessageActor (decode)
  -> EngineActor (route)
  -> OutgoingMessageActor (encode)
  -> Client WS frame
```

## Key dependencies

| Crate | Purpose |
|---|---|
| `crabby-specs` | Shared WebSocket message types (`CrabbyWsFromClient`, `CrabbyWsFromServer`) |
| `crabby-core` | Shutdown signal, token verification traits |
| `kameo` | Actor runtime |
| `ferroid` | Snowflake ID generation for message IDs |

## Binaries

- **`client`** — Interactive CLI WebSocket client for manual testing.
