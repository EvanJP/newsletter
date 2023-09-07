# Latest as of 06/09/23.
FROM rust:1.72.0

# Change to /app
WORKDIR /app

# Install required system dependencies for linking.

RUN apt update && apt install lld clang -y

# Copy all files to Docker image.
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release
ENTRYPOINT [ "./target/release/newsletter" ]



