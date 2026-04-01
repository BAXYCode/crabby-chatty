# crabby-auth

gRPC authentication service handling user registration, login, and token management.

## API (`auth.proto`)

| RPC | Description |
|---|---|
| `Register` | Create a new user (password hashed with Argon2) |
| `Login` | Validate credentials, return bearer + refresh PASETO tokens |
| `Refresh` | Issue a new bearer token using a valid refresh token |
| `PublicKey` | Return the asymmetric public key so other services can verify tokens locally |

Served via Tonic on port `6769`.

## Internals

- **PASETO v4 tokens** — Asymmetric (public/secret key pair). Keys are stored in Postgres via `PasetoKeyRepo`.
- **Argon2 password hashing** — A single static `Argon2` instance is reused across requests.
- **User storage** — `UserRepo` trait backed by `PostgresUserRepo` (sqlx).
- **gRPC interceptor** — Extracts bearer tokens from the `Authorization` header for the `Refresh` flow.

## Running tests

To test this crate, you can use `just run` followed by `just test-all`.

If you do not have `just` installed, you can follow [this](https://github.com/casey/just) link to download it or open the Justfile and run the commands directly.

After the tests are done running, use `just clean` to reset the environment.
