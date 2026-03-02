
DB_URL := env_var_or_default("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/auth")
DB_WAIT_TIMEOUT := env_var_or_default("DB_WAIT_TIMEOUT", "45")

default:
    just --list

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

