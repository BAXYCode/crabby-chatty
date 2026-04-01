# crabby-group

Group management service exposing both a REST API (for CRUD) and a gRPC API (for fast inter-service membership queries).

## APIs

### REST (Axum + utoipa)

| Method | Path | Description |
|---|---|---|
| POST | `/group` | Create a new group with initial members |

OpenAPI docs are generated via `utoipa`.

### gRPC (`groups.proto`)

| RPC | Description |
|---|---|
| `CheckMembership` | Check whether a user belongs to any group |
| `ListGroupMembers` | Fetch members of a specific group (with version) |
| `BatchListGroupMembers` | Bulk-query members for multiple groups |
| `GetGroupMembershipVersion` | Version number for cache-invalidation |

Both transports are served on the same port (default `:8080`, configurable via `HTTP_ADDR`).

## Data model

- **Roles**: `Admin`, `Member`
- **Events**: `Added`, `Removed`, `Left`

Persistence is PostgreSQL via `sqlx`. The `DatabaseRepo` trait abstracts storage, making it straightforward to swap or mock in tests.

## Testing

Integration tests use `testcontainers` to spin up a real Postgres instance — no mocks for the database layer.
