# crabby-fanout

Message fanout service that receives chat messages from a NATS subject, resolves recipients, and publishes per-user deliveries back to NATS.

## Architecture

The service is built on the [kameo](https://docs.rs/kameo) actor framework and subscribes to two NATS subjects:

- **Fanout message stream** — Incoming `CrabbyWsFromServer` messages. For individual messages the recipient is used directly; for group messages the cached member list is used to fan out to each member (excluding the sender).
- **Group change event stream** — `GroupChangeId` events that trigger a gRPC call to `crabby-group` to refresh the cached member list for a group.

Outbound per-user messages are published to `user.{id}.delivery` NATS subjects via the `UserMessagePublisher` trait.

### Key modules

| Module | Purpose |
|---|---|
| `service` | `FanoutService` actor — message routing, group cache |
| `traits` | `GroupMembershipClient` and `UserMessagePublisher` abstractions |
| `grpc_client` | gRPC implementation of `GroupMembershipClient` |
| `nats_publisher` | NATS implementation of `UserMessagePublisher` |

### Key dependencies

| Crate | Purpose |
|---|---|
| `crabby-specs` | Shared message types, NATS channel definitions |
| `crabby-transport` | Abstract `Channel`, `Publisher`, `Subscriber`, `Codec` traits |
| `kameo` | Actor runtime |
| `tonic` | gRPC client for group membership lookups |

## Testing

The crate has NATS integration tests (`tests/fanout_nats_integration.rs`) that use a real NATS server with mocked gRPC group membership. These are `#[ignore]`d by default.

### Prerequisites

- Docker (for the NATS server)
- [just](https://github.com/casey/just) command runner (optional, for convenience)

### Running tests

**NATS integration tests:**

1. Start the NATS server:

```sh
just nats-up
```

2. Run the ignored tests:

```sh
just test-nats
```

3. Tear down when done:

```sh
just nats-down
```

Or run everything at once:

```sh
just test-all
```

### E2E tests

The full end-to-end test (`e2e/tests/group_chat.rs`) spins up the entire stack behind Traefik (group service, chat engine, fanout, NATS, Postgres) and exercises the group messaging flow over real WebSocket connections. The e2e commands live in the **workspace-root Justfile**.

```sh
# from the workspace root
just e2e-up     # build and start the stack
just test-e2e   # run the e2e test
just e2e-down   # tear down
just e2e        # all at once
```

### Just commands

| Command | Description |
|---|---|
| `just test` | Run unit tests (no external deps) |
| `just nats-up` | Start the NATS test server |
| `just nats-down` | Stop the NATS test server |
| `just test-nats` | Run only the NATS integration tests |
| `just test-all` | Start NATS, run all tests, tear down |
