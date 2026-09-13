# Purrce — Price Tracker

Aplicación self-hosted para monitorear precios de productos en múltiples tiendas, con historial, alertas y notificaciones. Backend en Rust/Axum y frontend en SvelteKit, empaquetados en una sola imagen Docker.

[![Docker Pulls](https://img.shields.io/docker/pulls/undaniel/purrce)](https://hub.docker.com/r/undaniel/purrce)
[![Docker Image Size](https://img.shields.io/docker/image-size/undaniel/purrce/latest)](https://hub.docker.com/r/undaniel/purrce)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](#licencia)

[English](README.md) · **Español**

## Características

- **Seguimiento de precios**: monitorea productos de varias tiendas y guarda su historial.
- **Alertas**: avísame cuando el precio baje de un umbral o un porcentaje.
- **Señales de compra**: detecta mínimos históricos y precios convenientes.
- **Multi-tienda**: compara ofertas del mismo producto entre distintas tiendas.
- **Tags y listas**: organiza productos por categoría, tienda o prioridad.
- **Notificaciones**: Telegram, ntfy, Email (SMTP) y Webhook.
- **Extracción automática**: cliente HTTP con impersonación + navegador headless (Obscura) para sitios con Cloudflare.
- **Interfaz moderna**: diseño Material 3 Expressive, tema claro/oscuro y responsive.
- **Bilingüe**: español si el navegador está en español, inglés para el resto.

---

## Capturas

<p align="center">
  <img src="docs/screenshots/dashboard.es.png" width="49%" alt="Dashboard" />
  <img src="docs/screenshots/products.es.png" width="49%" alt="Productos" />
</p>
<p align="center">
  <img src="docs/screenshots/product-detail.es.png" width="49%" alt="Detalle de producto" />
  <img src="docs/screenshots/alerts.es.png" width="49%" alt="Alertas" />
</p>
<p align="center">
  <img src="docs/screenshots/settings.es.png" width="49%" alt="Configuración" />
</p>

---

## Inicio rápido (Docker Hub)

La forma recomendada de instalarlo en tu equipo o servidor.

### Requisitos

- Docker Engine 24+ y Docker Compose v2.
- ~512 MB de RAM libres para el contenedor de la app (más PostgreSQL).

### 1. Crea una carpeta y el `compose.yaml`

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

### 2. Define tus variables

Crea un archivo `.env` junto al `compose.yaml`:

```env
POSTGRES_PASSWORD=cambia-esta-clave
DEFAULT_CURRENCY=CLP
```

> `DEFAULT_CURRENCY` es la moneda usada cuando no se puede detectar (ISO 4217: `CLP`, `USD`, `EUR`, `MXN`, …).

### 3. Levanta la aplicación

```bash
docker compose up -d
```

Abre **http://localhost:8080**.

Para ver logs:

```bash
docker compose logs -f purrce
```

---

## Persistencia y respaldo

| Volumen | Contenido |
|---|---|
| `postgres_data` | Base de datos (productos, ofertas, historial, alertas, configuración). |
| `obscura_data` | Perfil del navegador headless (cookies, sesiones, `cf_clearance`). |

Respaldo de la base de datos:

```bash
docker compose exec postgres pg_dump -U purrce purrce > purrce-backup-$(date +%F).sql
```

Restauración:

```bash
cat purrce-backup-2026-01-01.sql | docker compose exec -T postgres psql -U purrce purrce
```

---

## Configuración

Variables de entorno del contenedor `purrce`:

| Variable | Descripción | Por defecto |
|---|---|---|
| `DATABASE_URL` | Cadena de conexión a PostgreSQL | Requerida |
| `DEFAULT_CURRENCY` | Moneda por defecto (ISO 4217) | `CLP` |
| `RUST_LOG` | Nivel de logs | `purrce_backend=info` |
| `PROFILE` | Esfuerzo del scraper: `minimal` / `balanced` / `aggressive` | `balanced` |
| `SCHEDULER_INTERVAL` | Intervalo del planificador (segundos) | `30` |
| `OBSCURA_TIMEOUT` | Timeout del navegador headless (segundos) | `30` |
| `OBSCURA_MAX_CONCURRENCY` | Máximo de navegadores simultáneos | `3` |
| `OBSCURA_V8_FLAGS` | Flags de V8 para Obscura | `--max-old-space-size=256` |
| `OBSCURA_SCRIPT_DEADLINE_MS` | Límite de ejecución de scripts por página (ms) | `20000` |
| `HTTP_PROXY` / `HTTPS_PROXY` | Proxy HTTP(S) opcional | - |
| `PROXY_URL` | Proxy SOCKS5 opcional | - |

Perfiles del scraper:

- **minimal** — solo HTTP local, sin proxy ni reintentos con navegador (mínimo consumo).
- **balanced** — respeta los proxies y reintenta con navegador stealth (por defecto).
- **aggressive** — todos los intentos usan navegador stealth (más lento, más compatible).

---

## Exponer en internet (HTTPS)

Se recomienda un reverse proxy delante de Purrce. Ejemplo con [Caddy](https://caddyserver.com/):

```caddyfile
precios.tudominio.com {
    reverse_proxy localhost:8080
}
```

Caddy gestiona el certificado TLS automáticamente. Con Nginx o Traefik el patrón es el mismo: proxy a `localhost:8080`.

> No expongas el puerto de PostgreSQL (`5432`) a internet. En el `compose.yaml` no se publica, así que solo es accesible dentro de la red de Docker.

---

## Actualizar

```bash
docker compose pull
docker compose up -d
```

Las migraciones de base de datos se aplican automáticamente al iniciar el contenedor.

---

## Notificaciones

Se configuran desde la interfaz, en **Configuración → Canales de notificación**, y se guardan en la base de datos.

### Telegram

1. Crea un bot con [@BotFather](https://t.me/BotFather) (`/newbot`) y copia el token.
2. Obtén tu Chat ID con [@userinfobot](https://t.me/userinfobot). Para grupos, agrega [@RawDataBot](https://t.me/RawDataBot).
3. En la app, activa Telegram, ingresa el token y el Chat ID, y pulsa **Probar**.

### ntfy

1. Instala la [app de ntfy](https://ntfy.sh/#install) en tu teléfono.
2. En la app, activa ntfy y elige un tema (por ejemplo `mis-alertas-precio`).
3. Suscríbete al mismo tema en la app de ntfy y pulsa **Probar**.

### Email (SMTP)

1. Activa Email e ingresa host, puerto, remitente y destinatario.
2. Si tu proveedor lo requiere, añade usuario y contraseña.

### Webhook

1. Activa Webhook e ingresa la URL (Slack, Discord, n8n, Zapier, …).

---

## Desarrollo local

### Requisitos

- Rust 1.88+
- Node.js 20+
- Docker y Docker Compose

### Pasos

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

### Construir la imagen localmente

```bash
docker compose build
```

El `docker-compose.yml` del repositorio usa `build: .` para desarrollo. Para producción, usa la imagen publicada en Docker Hub.

---

## Arquitectura

```
┌─────────────────────────────────────┐
│          Contenedor Purrce          │
│  ┌───────────────────────────────┐  │
│  │       Servidor Rust/Axum      │  │
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

La imagen Docker se publica para `linux/amd64`.

## Stack

| Componente | Tecnología |
|---|---|
| Backend | Rust, Axum, SQLx |
| Frontend | SvelteKit, Tailwind CSS |
| Base de datos | PostgreSQL 16 |
| Scraping | Reqwest + Obscura (headless) |
| Contenedor | Docker, Debian Bookworm |

## API

### Productos
- `GET /api/products` — Listar productos
- `POST /api/products/import` — Importar producto desde URL
- `GET /api/products/{id}` — Detalle del producto
- `PUT /api/products/{id}` — Actualizar producto
- `DELETE /api/products/{id}` — Eliminar producto
- `GET /api/products/{id}/comparison` — Comparar precios entre tiendas
- `GET /api/products/{id}/tags` — Tags del producto
- `POST /api/products/{id}/tags` — Definir tags del producto

### Ofertas
- `GET /api/offers/{id}` — Detalle de la oferta
- `GET /api/offers/{id}/history` — Historial de precios
- `PATCH /api/offers/{id}/price` — Actualizar precio manualmente

### Monitoreos
- `GET /api/watches` — Listar monitoreos
- `POST /api/watches` — Crear monitoreo
- `PATCH /api/watches/{id}` — Actualizar monitoreo
- `DELETE /api/watches/{id}` — Eliminar monitoreo
- `POST /api/watches/{id}/check` — Verificar precio manualmente

### Alertas
- `GET /api/alerts` — Listar alertas
- `PATCH /api/alerts/{id}` — Actualizar alerta

### Tags
- `GET /api/tags` — Listar tags
- `POST /api/tags` — Crear tag
- `DELETE /api/tags/{id}` — Eliminar tag

### Notificaciones
- `GET /api/notifications/config` — Obtener configuración
- `PUT /api/notifications/config` — Actualizar configuración
- `POST /api/notifications/test/telegram` — Probar Telegram
- `POST /api/notifications/test/ntfy` — Probar ntfy

### Salud
- `GET /api/health` — Health check

## Contribuir

1. Haz fork del repositorio.
2. Crea una rama (`git checkout -b feature/mi-mejora`).
3. Haz commit de tus cambios.
4. Abre un Pull Request.

## Licencia

Distribuido bajo la licencia MIT. Ver [LICENSE](LICENSE).
