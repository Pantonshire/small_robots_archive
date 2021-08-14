FROM rust:1.54-alpine as build
WORKDIR /app/
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN apk update
RUN apk add --no-cache musl-dev
RUN cargo build --release --no-default-features

FROM alpine:latest as runtime
COPY --from=build /app/target/release/sbb_archive /usr/local/bin/sbb_archive
WORKDIR /srv/www/
COPY static/ ./static/
RUN mkdir -p generated/bootstrap
RUN mkdir -p generated/robot_images
RUN apk update
RUN apk add --no-cache libcap
RUN setcap 'cap_net_bind_service=+ep' /usr/local/bin/sbb_archive
EXPOSE 80
ENV BIND_ADDRESS="0.0.0.0:80"
RUN addgroup -S archive
RUN adduser -SDH -G archive archive
USER archive
ENTRYPOINT ["/usr/local/bin/sbb_archive"]
