# crabby-transport

Generic abstractions for message-oriented communication patterns (pub/sub).

## Traits

- **`Channel`** — Defines a named channel with associated `Message` and `Codec` types, plus a `subject()` for topic/routing-key selection.
- **`Codec<M>`** — Encode/decode messages to/from `Bytes`. Ships with a `JsonCodec` implementation using serde_json.
- **`Publisher<C: Channel>`** — Async `publish(message)` interface.
- **`Subscriber<C: Channel>`** — Async `subscribe(topic)` interface returning a stream.

## Design intent

This crate provides the abstraction layer for a future message-bus integration (e.g. NATS, Kafka, Redis Streams). Concrete implementations can be swapped in behind the traits without changing the services that depend on them.

Currently used by `crabby-specs` for its codec definitions.
