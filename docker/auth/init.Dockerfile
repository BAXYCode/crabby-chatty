
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest
USER root
COPY ../../crabby-auth/target/release/init_db /tool
COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./tool" ]
