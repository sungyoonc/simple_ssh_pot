# Stage - Build
FROM rust:1.75-alpine as build_stage

WORKDIR /usr/src/simple_ssh_pot
COPY . .

RUN apk add pkgconf openssl-dev musl-dev

# Use statically linked executables
RUN RUSTFLAGS="-Ctarget-feature=-crt-static" cargo build --release

# Stage - Deploy
FROM alpine:3.19 as deploy_stage

WORKDIR /app
RUN apk add libgcc

COPY --from=build_stage /usr/src/simple_ssh_pot/target/release/simple_ssh_pot /app/simple_ssh_pot

ENTRYPOINT ["/app/simple_ssh_pot"]
