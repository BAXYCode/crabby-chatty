
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest
USER root
RUN apt-get -y update; apt-get -y install curl
COPY ./../target/release/init_db /tool
COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./tool" ]
