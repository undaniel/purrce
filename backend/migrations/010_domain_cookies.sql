-- Cookies persisted per origin host so a solved challenge (cf_clearance,
-- datadome, etc.) survives restarts and keeps later checks cheap.
CREATE TABLE IF NOT EXISTS domain_cookies (
    host       TEXT NOT NULL,
    name       TEXT NOT NULL,
    path       TEXT NOT NULL,
    value      TEXT NOT NULL,
    domain     TEXT,
    expires_at TIMESTAMPTZ,
    secure     BOOLEAN NOT NULL DEFAULT false,
    http_only  BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (host, name, path)
);

CREATE INDEX IF NOT EXISTS idx_domain_cookies_expires ON domain_cookies(expires_at);
