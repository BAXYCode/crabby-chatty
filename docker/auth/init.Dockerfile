
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest
USER root
COPY ../../crabby-infra/target/release/init_db /tool
COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./tool" ]
