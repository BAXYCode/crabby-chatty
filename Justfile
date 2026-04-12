default:
    @just --list

# Build all service binaries in release mode
build:
    cargo build --release -p crabby-chat -p crabby-group -p crabby-fanout

# Build binaries, then build and start the full e2e stack
e2e-up: build
    docker compose -f docker/docker-compose.e2e.yml up -d --build

# Tear down the e2e stack
e2e-down:
    docker compose -f docker/docker-compose.e2e.yml down -v

# Run the e2e tests (requires `just e2e-up` first)
test-e2e:
    cargo test -p e2e -- --ignored

# Full e2e cycle: build, start, test, tear down
e2e: e2e-up
    cargo test -p e2e -- --ignored
    @just e2e-down

DB_URL := env_var_or_default("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/auth")
DB_WAIT_TIMEOUT := env_var_or_default("DB_WAIT_TIMEOUT", "45")

up-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up --remove-orphans

auth-down:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml down

auth:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up -d --remove-orphans

dns_down:
    sudo systemctl stop systemd-resolved

dns_up:
    sudo systemctl start systemd-resolved
    

auth-up:
  docker compose -f docker/docker-compose.yml up -d postgres

auth-wait-db:
  ./scripts/wait-for-postgres.sh "{{DB_URL}}" "{{DB_WAIT_TIMEOUT}}"

auth-run: auth-up auth-wait-db
  cargo run -p crabby-auth --release
