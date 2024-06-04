
FROM cockroachdb/cockroach:latest as cockroach
FROM ubuntu:latest
USER root
RUN apt-get -y update; apt-get -y install curl; apt-get -y install inetutils-ping
COPY ./../target/release/init_db /tool
COPY --from=cockroach /cockroach/ ./
ENTRYPOINT [ "./tool" ]
