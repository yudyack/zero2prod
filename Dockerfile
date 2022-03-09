FROM lukemathwalker/cargo-chef:latest as chef
WORKDIR /app
# Install OpenSSL - it is dynamically linked by some of our dependencies
RUN apt-get update -y \
    && apt-get install -y pkg-config libssl-dev \
    && apt-get install -y build-essential git clang cmake libstdc++-10-dev libssl-dev libxxhash-dev zlib1g-dev pkg-config \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

RUN git clone https://github.com/rui314/mold.git -b v1.1.1
RUN make -C mold -j$(nproc) CXX=clang++
RUN make -C mold install

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# We use the latest Rust stable release as base image
FROM chef AS builder
COPY --from=mold /mold/mold /usr/bin/mold
COPY --from=mold /mold/mold-wrapper.so /usr/bin/mold-wrapper.so
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN mold --run cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.
COPY . .
ENV SQLX_OFFLINE true
# build out project
COPY migrations migrations

COPY --from=mold /mold/mold /usr/bin/mold
COPY --from=mold /mold/mold-wrapper.so /usr/bin/mold-wrapper.so

RUN mold --run cargo build --release --bin zero2prod

# runtime stage
FROM debian:bullseye-slim AS runtime

WORKDIR /app
# Install OpenSSL - it is dynamically linked by some of our dependencies
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates\
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]