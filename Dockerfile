FROM rust:1.54-alpine as build
WORKDIR /app/
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN apk update \
    && apk add --no-cache musl-dev
RUN cargo build --release --no-default-features

FROM alpine:latest as runtime
COPY --from=build /app/target/release/sbb_archive /usr/local/bin/sbb_archive
WORKDIR /srv/www/
COPY static/ ./static/
RUN mkdir -p generated
EXPOSE 8080
ENV BIND_ADDRESS="0.0.0.0:8080"
ENV USER_ID=12001 GROUP_ID=12001
RUN addgroup -S -g "$GROUP_ID" archive \
    && adduser -SDH -u "$USER_ID" -g archive archive
USER archive
ENTRYPOINT ["/usr/local/bin/sbb_archive"]
