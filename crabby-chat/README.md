# crabby-chat

Real-time messaging service built on Axum WebSockets and the [kameo](https://docs.rs/kameo) actor framework.

## Architecture

The service exposes a single WebSocket endpoint (`:6969/ws`) and uses an actor-based design for concurrency:

- **`EngineActor`** — Central hub that tracks all connected users (`HashMap<Uuid, Recipient>`) and routes messages between them. Publishes outbound messages to a NATS fanout subject and receives per-user deliveries via an attached NATS subscriber stream.
- **`IncomingMessageActor`** — Reads raw WebSocket frames, decodes them into domain types via the `Decode` trait, and forwards them to the engine.
- **`OutgoingMessageActor`** — Encodes domain messages into binary WebSocket frames (via `ServerToTransport` / `Encode`) and writes them to the client sink.

### Message flow

```
Client WS frame
  -> IncomingMessageActor (decode)
  -> EngineActor (publish to NATS fanout)
  -> [future fanout service]
  -> NATS user.{id}.delivery
  -> EngineActor (delivery stream)
  -> OutgoingMessageActor (encode)
  -> Client WS frame
```

## Key dependencies

| Crate | Purpose |
|---|---|
| `crabby-specs` | Shared message types, NATS channel definitions, delivery stream adapter |
| `crabby-transport` | Abstract `Channel`, `Publisher`, `Subscriber`, `Codec` traits |
| `crabby-core` | Shutdown signal, token verification traits |
| `kameo` | Actor runtime |
| `ferroid` | Snowflake ID generation for message IDs |

## Binaries

- **`client`** — Interactive CLI WebSocket client for manual testing.

## Testing

The crate has two test suites:

- **Engine integration tests** (`tests/engine_integration.rs`) — Test actor message handling, user connect/disconnect, delivery routing, and stream attachment. No external dependencies required.
- **NATS integration tests** (`tests/nats_integration.rs`) — End-to-end tests that publish and subscribe through a real NATS server. These are `#[ignore]`d by default and require a running NATS instance.

### Prerequisites

- Docker (for NATS integration tests)
- [just](https://github.com/casey/just) command runner (optional, for convenience)

### Running tests

**Unit + engine integration tests** (no external deps):

```sh
cargo test -p crabby-chat
# or
just test
```

**NATS integration tests:**

1. Start the NATS server:

```sh
docker compose -f docker/docker-compose.test.yml up -d
```

2. Run the ignored tests:

```sh
cargo test -p crabby-chat --test nats_integration -- --ignored
```

3. Tear down when done:

```sh
docker compose -f docker/docker-compose.test.yml down
```

Or use `just` to do it all at once:

```sh
just test-all
```

The NATS server connects on `localhost:4222` by default. Override with the `NATS_URL` environment variable if needed:

```sh
NATS_URL=nats://some-other-host:4222 cargo test -p crabby-chat --test nats_integration -- --ignored
```

### Just commands

| Command | Description |
|---|---|
| `just test` | Run unit + engine integration tests |
| `just nats-up` | Start the NATS test server |
| `just nats-down` | Stop the NATS test server |
| `just test-nats` | Run only the NATS integration tests |
| `just test-all` | Start NATS, run all tests, tear down |
