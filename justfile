default:
    just --list

up-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up --remove-orphans

down-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml down

init-insecure: down-insecure && up-insecure
    just comp-init

comp-init:
    cd crabby-infra && cargo build --bin init_db --release
    docker build -f docker/auth/init.Dockerfile -t baxydocker/db-init:latest .

up-sec:
    cd docker/auth && docker compose -f docker-compose.secure.yaml up --remove-orphans

down-sec:
    cd docker/auth && docker compose -f docker-compose.secure.yaml down

init-secure: down-sec && up-sec
    just  comp-certs

comp-certs:
    cd crabby-infra && cargo build --bin certs_gen --release
    docker build -f docker/auth/certs_gen.Dockerfile -t baxydocker/certs_gen:latest .

auth:
    cd docker/auth && docker compose -f docker-compose.secure.yaml up -d --remove-orphans

dns_down:
    sudo systemctl stop systemd-resolved

dns_up:
    sudo systemctl start systemd-resolved
migrate-init:
    cd init_db && sqlx migrate run --database-url "postgres://root@localhost:36257/?sslmode=verify-full&sslcert=../docker/volumes/certs/client.root.crt&sslkey=../docker/volumes/certs/client.root.key&sslrootcert=../docker/volumes/certs/ca.crt"
