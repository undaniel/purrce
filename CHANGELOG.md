# Changelog

All notable changes to Purrce are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-09-13

First public release.

### Added

- Self-hosting documentation with Docker Hub and data backup instructions.
- Screenshots section in the README (`docs/screenshots/`).
- **Price tracking**: import products by URL, keep price history, and compare prices across stores.
- **Buy signals**: detect historical lows, range position, and savings versus the average.
- **Alerts**: threshold-based and percentage-change based.
- **Watches**: scheduled automatic checks, pause/resume, and manual verification.
- **Tags and lists**: organize and group products.
- **Notifications**: Telegram, ntfy, Email (SMTP), and Webhook, with a test button.
- **Automatic extraction**: HTTP client with impersonation and a headless browser (Obscura) for Cloudflare-protected sites, with a configurable effort profile (`minimal` / `balanced` / `aggressive`).
- **Proxy support**: HTTP(S) and SOCKS5.
- **Multi-currency support** with per-offer currency formatting.
- **Live events** over SSE (real-time price updates and alerts).
- **ES/EN internationalization**: the app detects the browser language (Spanish if the browser is in Spanish, English otherwise), with translated dates, numbers, signals, and statuses.
- **Material 3 Expressive UI**: floating navigation rail, mobile toolbar, buttons, inputs, dialogs, cards, badges, tables, and type scale.
- **Light/dark theme** and responsive design.
- **Favicon** with the brand identity.
- **Unified Docker image** (backend + frontend + Obscura) compatible with amd64 and arm64, with a healthcheck.
- **Automatic database migrations** on startup.

### Security

- Headless browser cookies and sessions persisted to reuse solved challenges.
- Container isolation with an unprivileged user.
- PostgreSQL is not published outside the Docker network by default.

[Unreleased]: https://github.com/undaniel/purrce/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/undaniel/purrce/releases/tag/v1.0.0
