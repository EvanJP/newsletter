# Latest as of 06/09/23.
# cargo chef allows pre-building dependencies.
FROM lukemathwalker/cargo-chef:latest-rust-1.72.0-slim as chef

# FROM debian:bullseye-slim AS runtime
WORKDIR /app

RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Cache dependencies and build binary.
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Since recipe.json stays the same, this gets cached by Docker.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . . 
ENV SQLX_OFFLINE true
RUN cargo build --release --bin newsletter

FROM debian:bullseye-slim AS runtime
RUN apt-get update -y  \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \ 
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/newsletter newsletter
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./newsletter"]