FROM rust:1.71

WORKDIR /usr/src/listen_ssh
COPY . .

RUN cargo install --path .

CMD ["listen_ssh"]
