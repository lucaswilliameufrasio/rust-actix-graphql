# Rust-Actix-Graphql ![tests](https://github.com/lucaswilliameufrasio/rust-actix-graphql/workflows/tests/badge.svg)
Blog made in actix following [Genus-V Programming](https://www.youtube.com/watch?v=9q4GcWbAIEM) Youtube tutorial.

## Requirements
- Rust
- Docker
- docker-compose

## Usage
```
# Copy example .env file
cp .env.example .env

# Run postgres
docker-compose up -d postgres

# Install diesel
cargo install diesel_cli --no-default-features --features postgres

# Run db migrations
DATABASE_URL=postgres://actix:actix@localhost:5432/actix diesel migration run

# Run unit tests
cargo test

# Run the server (Add --release for an optimized build)
cargo run 
```
```
curl -s http://localhost:8080/health
```
