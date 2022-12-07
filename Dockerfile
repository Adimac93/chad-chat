# Frontend
FROM node:19-bullseye-slim AS frontend
WORKDIR /app

COPY /frontend/package.json .
COPY /frontend/package-lock.json .
RUN npm install
COPY frontend .
RUN npm run build

# Backend
FROM lukemathwalker/cargo-chef:latest-rust-1.65.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY backend .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY backend .
ENV SQLX_OFFLINE true
# Build our project
RUN cargo build --release --bin backend

FROM debian:bullseye-slim AS runtime
WORKDIR /app/
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backend ./backend/backend
COPY --from=frontend /app/dist ./frontend/dist
ENV APP_ENVIRONMENT production

WORKDIR /app/backend
ENTRYPOINT ["./backend"]