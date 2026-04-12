FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY target/release/crabby-group /usr/local/bin/
CMD ["crabby-group"]
