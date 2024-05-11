
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest

COPY ./../target/release/init_db /tool
COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./tool" ]
