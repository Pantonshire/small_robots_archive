# Build stage: compile the binary for the runtime stage to use
FROM rust:1.54-alpine as build
WORKDIR /app/
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
RUN apk update
# Install musl development files, which the paste crate links
RUN apk add --no-cache musl-dev
RUN cargo build --release --no-default-features

# Runtime stage: set up the environment needed to run the binary
FROM alpine:latest as runtime
COPY --from=build /app/target/release/sbb_archive /usr/local/bin/sbb_archive
WORKDIR /srv/www/
COPY static/ ./static/
RUN mkdir -p generated/robot_images
ENV BIND_ADDRESS="0.0.0.0:80"
EXPOSE 80
ENTRYPOINT ["/usr/local/bin/sbb_archive"]
