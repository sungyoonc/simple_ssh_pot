FROM rust:1.75-alpine

WORKDIR /usr/src/simple_ssh_pot
COPY . .

RUN apk add pkgconf openssl-dev musl-dev

# Use statically linked executables
RUN RUSTFLAGS="-Ctarget-feature=-crt-static" cargo install --path .

CMD ["simple_ssh_pot"]
