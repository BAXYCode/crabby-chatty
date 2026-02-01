FROM rust:bullseye as init

WORKDIR /src/crabby-chat

COPY . .
COPY source dest
RUN ls -a
RUN cargo build --release --bin crabby-chat 


FROM scratch 

WORKDIR /service

COPY --from=init ./src/crabby-chat/target/release/crabby-chat /service/crabby-chat


CMD [ "/service/crabby-chat" ]
