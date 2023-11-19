FROM rust:1.54-alpine as planner
WORKDIR /app/
RUN apk update && apk add --no-cache musl-dev
RUN cargo install cargo-chef && rm -rf /usr/local/cargo/registry/
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.54-alpine as cacher
WORKDIR /app/
RUN apk update && apk add --no-cache musl-dev
RUN cargo install cargo-chef && rm -rf /usr/local/cargo/registry/
COPY --from=planner /app/recipe.json ./recipe.json
RUN cargo chef cook --release --no-default-features --recipe-path recipe.json

FROM rust:1.54-alpine as builder
WORKDIR /app/
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --no-default-features

FROM alpine:latest as runtime
COPY --from=builder /app/target/release/sbb_archive /usr/local/bin/sbb_archive
WORKDIR /srv/www/
COPY static/ ./static/
RUN mkdir -p generated
EXPOSE 8080
ENV BIND_ADDRESS="0.0.0.0:8080"
ARG USER_ID=12001 GROUP_ID=12001
RUN addgroup -S -g "$GROUP_ID" archive && adduser -SDH -u "$USER_ID" -g archive archive
USER archive
ENTRYPOINT ["/usr/local/bin/sbb_archive"]
