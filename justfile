default:
    just --list

up-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up --remove-orphans

down-insecure:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml down

auth:
    cd docker/auth && docker compose -f docker-compose.insecure.yaml up -d --remove-orphans

dns_down:
    sudo systemctl stop systemd-resolved

dns_up:
    sudo systemctl start systemd-resolved
