FROM rust:latest as init

WORKDIR /src/inter
# RUN sudo apt-get update && sudo apt-get upgrade
COPY ./ ./
RUN  file="$(ls -a)" && echo $file
RUN cargo build --release --package crabby-chat 




FROM ubuntu:jammy 

WORKDIR /service/

COPY --from=init /src/inter/target/release/crabby-chat /service/crabby-chat
# RUN echo * 

CMD ./crabby-chat
