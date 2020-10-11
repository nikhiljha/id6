FROM rust:1.47 as builder
WORKDIR /usr/src/id6
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/id6 /usr/local/bin/id6
CMD ["id6"]
