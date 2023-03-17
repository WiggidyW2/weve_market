FROM rust:1.68-bullseye
WORKDIR /root/weve_market/
COPY ./src src
COPY ./proto proto
COPY ./build.rs .
COPY ./Cargo.toml .
RUN apt update
RUN apt install protobuf-compiler -y
RUN cargo build --release

FROM frolvlad/alpine-glibc:alpine-3.17
WORKDIR /root/
COPY --from=0 /root/weve_market/target/release/weve_market bin
RUN chmod +x ./bin
CMD ["./bin"]
