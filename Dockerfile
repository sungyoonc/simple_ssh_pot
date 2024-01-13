FROM rust:1.75

WORKDIR /usr/src/simple_ssh_pot
COPY . .

RUN cargo install --path .

CMD ["simple_ssh_pot"]
