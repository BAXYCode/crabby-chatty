
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest
USER root
COPY ../../crabby-infra/target/release/certs_gen /gen
RUN mkdir -pv ./.cockroach-certs
RUN mkdir -pv ./.cockroach-key

COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./gen" ]
