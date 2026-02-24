FROM rust:1.75-alpine AS builder

RUN apk add --no-cache musl-dev postgresql-dev

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM alpine:latest

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/target/release/api-gateway-simple /usr/local/bin/api-gateway-simple

EXPOSE 8081

CMD ["api-gateway-simple"]
