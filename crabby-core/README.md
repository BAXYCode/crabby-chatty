# crabby-core

Shared foundation crate providing cross-cutting concerns used by the other crabby-chatty services.

## What's in here

- **`Engine` trait** — Async `run()` interface for bootstrapping services.
- **`shutdown_signal()`** — Listens for `Ctrl+C` / `SIGTERM` and returns a future that resolves on either, enabling graceful shutdown in any Tokio-based service.
- **`VerifyToken` / `KeyRetrieval` traits** — Abstractions for PASETO token verification and public-key retrieval, consumed by services that need to authenticate incoming requests.

## Design intent

`crabby-core` deliberately contains **no business logic**. It exists so that every service can share a consistent approach to startup, shutdown, and token handling without pulling in heavy dependencies from one another.
