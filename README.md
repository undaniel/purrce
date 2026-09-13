# Purrce — Price Tracker

Self-hosted app to track product prices across multiple stores, with history, alerts, and notifications. Rust/Axum backend and SvelteKit frontend, packaged in a single Docker image.

[![Docker Pulls](https://img.shields.io/docker/pulls/undaniel/purrce)](https://hub.docker.com/r/undaniel/purrce)
[![Docker Image Size](https://img.shields.io/docker/image-size/undaniel/purrce/latest)](https://hub.docker.com/r/undaniel/purrce)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](#license)

**English** · [Español](README.es.md)

## Features

- **Price tracking**: monitor products from multiple stores and keep their price history.
- **Alerts**: notify me when a price drops below a threshold or a percentage.
- **Buy signals**: detect historical lows and good deals.
- **Multi-store**: compare offers for the same product across stores.
- **Tags and lists**: organize products by category, store, or priority.
- **Notifications**: Telegram, ntfy, Email (SMTP), and Webhook.
- **Automatic extraction**: HTTP client with impersonation + headless browser (Obscura) for Cloudflare-protected sites.
- **Modern UI**: Material 3 Expressive design, light/dark theme, and responsive layout.
- **Bilingual**: Spanish if the browser is in Spanish, English for everyone else.

---

## Screenshots

<p align="center">
  <img src="docs/screenshots/dashboard.png" width="49%" alt="Dashboard" />
  <img src="docs/screenshots/products.png" width="49%" alt="Products" />
</p>
<p align="center">
  <img src="docs/screenshots/product-detail.png" width="49%" alt="Product detail" />
  <img src="docs/screenshots/alerts.png" width="49%" alt="Alerts" />
</p>
<p align="center">
  <img src="docs/screenshots/settings.png" width="49%" alt="Settings" />
</p>

---

## Quick start (Docker Hub)

The recommended way to install it on your computer or server.

### Requirements

- Docker Engine 24+ and Docker Compose v2.
- ~512 MB of free RAM for the app container (plus PostgreSQL).

### 1. Create a folder and the `compose.yaml`

```bash
mkdir purrce && cd purrce
```

`compose.yaml`:

```yaml
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
```

### 2. Set your variables

Create a `.env` file next to `compose.yaml`:

```env
POSTGRES_PASSWORD=change-this-password
DEFAULT_CURRENCY=CLP
```

> `DEFAULT_CURRENCY` is used when the currency cannot be detected (ISO 4217: `CLP`, `USD`, `EUR`, `MXN`, …).

### 3. Start the app

```bash
docker compose up -d
```

Open **http://localhost:8080**.

To follow the logs:

```bash
docker compose logs -f purrce
```

---

## Persistence and backup

| Volume | Contents |
|---|---|
| `postgres_data` | Database (products, offers, history, alerts, configuration). |
| `obscura_data` | Headless browser profile (cookies, sessions, `cf_clearance`). |

Back up the database:

```bash
docker compose exec postgres pg_dump -U purrce purrce > purrce-backup-$(date +%F).sql
```

Restore:

```bash
cat purrce-backup-2026-01-01.sql | docker compose exec -T postgres psql -U purrce purrce
```

---

## Configuration

Environment variables for the `purrce` container:

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `DEFAULT_CURRENCY` | Default currency (ISO 4217) | `CLP` |
| `RUST_LOG` | Log level | `purrce_backend=info` |
| `PROFILE` | Scraper effort: `minimal` / `balanced` / `aggressive` | `balanced` |
| `SCHEDULER_INTERVAL` | Scheduler interval (seconds) | `30` |
| `OBSCURA_TIMEOUT` | Headless browser timeout (seconds) | `30` |
| `OBSCURA_MAX_CONCURRENCY` | Max concurrent browser processes | `3` |
| `OBSCURA_V8_FLAGS` | V8 flags for Obscura | `--max-old-space-size=256` |
| `OBSCURA_SCRIPT_DEADLINE_MS` | Per-page script execution cap (ms) | `20000` |
| `HTTP_PROXY` / `HTTPS_PROXY` | Optional HTTP(S) proxy | - |
| `PROXY_URL` | Optional SOCKS5 proxy | - |

Scraper profiles:

- **minimal** — local HTTP only, no proxy, no browser retries (lowest resource usage).
- **balanced** — honors proxies and retries with the stealth browser (default).
- **aggressive** — every attempt uses the stealth browser (slower, most compatible).

---

## Exposing it to the internet (HTTPS)

Put a reverse proxy in front of Purrce. Example with [Caddy](https://caddyserver.com/):

```caddyfile
prices.yourdomain.com {
    reverse_proxy localhost:8080
}
```

Caddy handles the TLS certificate automatically. With Nginx or Traefik the pattern is the same: proxy to `localhost:8080`.

> Do not expose PostgreSQL (`5432`) to the internet. The `compose.yaml` above does not publish it, so it is only reachable inside the Docker network.

---

## Updating

```bash
docker compose pull
docker compose up -d
```

Database migrations run automatically when the container starts.

---

## Notifications

They are configured from the UI, under **Settings → Notification channels**, and stored in the database.

### Telegram

1. Create a bot with [@BotFather](https://t.me/BotFather) (`/newbot`) and copy the token.
2. Get your Chat ID with [@userinfobot](https://t.me/userinfobot). For groups, add [@RawDataBot](https://t.me/RawDataBot).
3. In the app, enable Telegram, enter the token and Chat ID, then click **Test**.

### ntfy

1. Install the [ntfy app](https://ntfy.sh/#install) on your phone.
2. In the app, enable ntfy and choose a topic (for example `my-price-alerts`).
3. Subscribe to the same topic in the ntfy app and click **Test**.

### Email (SMTP)

1. Enable Email and enter host, port, sender, and recipient.
2. Add username and password if your provider requires them.

### Webhook

1. Enable Webhook and enter the URL (Slack, Discord, n8n, Zapier, …).

---

## Local development

### Requirements

- Rust 1.88+
- Node.js 20+
- Docker and Docker Compose

### Steps

```bash
# 1. PostgreSQL
docker compose up postgres -d

# 2. Backend (terminal 1)
cd backend
cargo run

# 3. Frontend (terminal 2)
cd frontend
npm install
npm run dev
```

- Frontend (dev): http://localhost:5173
- Backend API: http://localhost:8080

### Build the image locally

```bash
docker compose build
```

The repository's `docker-compose.yml` uses `build: .` for development. For production, use the image published on Docker Hub.

---

## Architecture

```
┌─────────────────────────────────────┐
│           Purrce container          │
│  ┌───────────────────────────────┐  │
│  │        Rust/Axum server       │  │
│  │  ┌─────────┐  ┌───────────┐  │  │
│  │  │   API   │  │ Frontend  │  │  │
│  │  │ /api/*  │  │ SPA       │  │  │
│  │  └─────────┘  └───────────┘  │  │
│  └───────────────────────────────┘  │
│                  │                   │
│                  ▼                   │
│           ┌───────────┐             │
│           │  Obscura  │             │
│           │ (Headless)│             │
│           └───────────┘             │
└─────────────────────────────────────┘
                  │
                  ▼
           ┌───────────┐
           │ PostgreSQL │
           └───────────┘
```

The Dockerfile is multi-stage and compatible with `x86_64` (amd64) and `aarch64` (arm64).

## Stack

| Component | Technology |
|---|---|
| Backend | Rust, Axum, SQLx |
| Frontend | SvelteKit, Tailwind CSS |
| Database | PostgreSQL 16 |
| Scraping | Reqwest + Obscura (headless) |
| Container | Docker, Debian Bookworm |

## API

### Products
- `GET /api/products` — List products
- `POST /api/products/import` — Import product from URL
- `GET /api/products/{id}` — Product details
- `PUT /api/products/{id}` — Update product
- `DELETE /api/products/{id}` — Delete product
- `GET /api/products/{id}/comparison` — Compare prices across stores
- `GET /api/products/{id}/tags` — Product tags
- `POST /api/products/{id}/tags` — Set product tags

### Offers
- `GET /api/offers/{id}` — Offer details
- `GET /api/offers/{id}/history` — Price history
- `PATCH /api/offers/{id}/price` — Update price manually

### Watches
- `GET /api/watches` — List watches
- `POST /api/watches` — Create watch
- `PATCH /api/watches/{id}` — Update watch
- `DELETE /api/watches/{id}` — Delete watch
- `POST /api/watches/{id}/check` — Check price manually

### Alerts
- `GET /api/alerts` — List alerts
- `PATCH /api/alerts/{id}` — Update alert

### Tags
- `GET /api/tags` — List tags
- `POST /api/tags` — Create tag
- `DELETE /api/tags/{id}` — Delete tag

### Notifications
- `GET /api/notifications/config` — Get configuration
- `PUT /api/notifications/config` — Update configuration
- `POST /api/notifications/test/telegram` — Test Telegram
- `POST /api/notifications/test/ntfy` — Test ntfy

### Health
- `GET /api/health` — Health check

## Contributing

1. Fork the repository.
2. Create a branch (`git checkout -b feature/my-improvement`).
3. Commit your changes.
4. Open a Pull Request.

## License

Released under the MIT License. See [LICENSE](LICENSE).
