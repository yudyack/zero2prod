# We use the latest Rust stable release as base image
FROM rust:1.57.0 AS builder
WORKDIR /app
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release

# runtime stage
FROM rust:1.57.0 AS runtime

WORKDIR /app
# Copy the compiled binary from the builder environment
# to our runtime environment
COPY --from=builder /app/target/release/zero2prod zero2prod
# We need the configuration file at runtime!
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]