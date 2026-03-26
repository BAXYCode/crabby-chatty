FROM ubuntu:jammy

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /service
COPY target/release/crabby-group /service/crabby-group

CMD ["./crabby-group"]
