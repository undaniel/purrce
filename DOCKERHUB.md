# Purrce — Price Tracker

Self-hosted app to track product prices across multiple stores, with price history, alerts, and notifications. Rust/Axum backend and SvelteKit frontend in a single container.

## Features

- **Price tracking** — monitor products from multiple stores and keep their price history.
- **Alerts** — notify when a price drops below a threshold or a percentage.
- **Buy signals** — detect historical lows and good deals.
- **Multi-store** — compare offers for the same product across stores.
- **Tags and lists** — organize products by category, store, or priority.
- **Notifications** — Telegram, ntfy, Email (SMTP), and Webhook.
- **Automatic extraction** — HTTP client with impersonation + headless browser (Obscura) for Cloudflare-protected sites.
- **Modern UI** — Material 3 Expressive design, light/dark theme, responsive layout.
- **Bilingual** — Spanish if the browser is in Spanish, English otherwise.

## Quick start

`compose.yaml`:

    services:
      purrce:
        image: undaniel/purrce:latest
        restart: unless-stopped
        ports:
          - "8080:8080"
        environment:
          DATABASE_URL: postgres://purrce:${POSTGRES_PASSWORD}@postgres:5432/purrce
          RUST_LOG: purrce_backend=info,tower_http=info
          DEFAULT_CURRENCY: ${DEFAULT_CURRENCY:-CLP}
        volumes:
          - obscura_data:/app/data
        depends_on:
          postgres:
            condition: service_healthy

      postgres:
        image: postgres:16-alpine
        restart: unless-stopped
        environment:
          POSTGRES_USER: purrce
          POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
          POSTGRES_DB: purrce
        volumes:
          - postgres_data:/var/lib/postgresql/data
        healthcheck:
          test: ["CMD-SHELL", "pg_isready -U purrce"]
          interval: 5s
          timeout: 5s
          retries: 5

    volumes:
      postgres_data:
      obscura_data:

`.env`:

    POSTGRES_PASSWORD=change-this-password
    DEFAULT_CURRENCY=CLP

Start it:

    docker compose up -d

Open **http://localhost:8080**.

## Volumes

- `postgres_data` — database (products, offers, history, alerts, configuration).
- `obscura_data` — headless browser profile (cookies, sessions).

Backup:

    docker compose exec postgres pg_dump -U purrce purrce > purrce-backup-$(date +%F).sql

## Configuration

- `DATABASE_URL` — PostgreSQL connection string (required).
- `DEFAULT_CURRENCY` — default currency, ISO 4217 (default `CLP`).
- `PROFILE` — scraper effort: `minimal` / `balanced` / `aggressive` (default `balanced`).
- `RUST_LOG` — log level (default `purrce_backend=info`).
- `HTTP_PROXY` / `HTTPS_PROXY` / `PROXY_URL` — optional proxies.

## Updating

    docker compose pull
    docker compose up -d

Database migrations run automatically on startup.

## Platform

Image built for **linux/amd64**.

## Links

- Source code and docs: https://github.com/undaniel/purrce
- License: MIT
