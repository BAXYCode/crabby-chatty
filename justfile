up-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up --remove-orphans

down-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml down

init-insecure: down-insecure && up-insecure
    just comp-init

comp-init:
    cd /home/benjaminlaine/Documents/projects/crabby-chatty/crabby-auth/custom-images && cargo build --bin init_db --release
    docker build -f docker/auth/init.Dockerfile -t baxydocker/db-init:latest .

up-sec:
    cd docker/auth && docker compose -f docker-compose.secure.yaml up --remove-orphans

down-sec:
    cd docker/auth && docker compose -f docker-compose.secure.yaml down

init-secure: down-sec && up-sec
    just  comp-certs

comp-certs:
    cd /home/benjaminlaine/Documents/projects/crabby-chatty/crabby-auth/custom-images && cargo build --bin certs_gen --release
    docker build -f docker/auth/certs_gen.Dockerfile -t baxydocker/certs_gen:latest .

auth:
    cd docker/auth && docker compose -f docker-compose.secure.yaml up -d --remove-orphans

dns_down:
    sudo systemctl stop systemd-resolved

dns_up:
    sudo systemctl start systemd-resolved
