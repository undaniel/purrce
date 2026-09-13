//! Persistent cookie storage keyed by origin host.
//!
//! wreq's built-in `Jar` is RFC-compliant but its `get_all()` snapshot drops the
//! host for host-only cookies, which are exactly the interesting ones
//! (`cf_clearance`, `datadome`). So we wrap the jar and mirror every
//! `Set-Cookie` into a pending list at response time, where the request URI is
//! still available, and flush that to Postgres.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use chrono::{DateTime, Utc};
use http::{HeaderValue, Uri, Version};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{debug, warn};
use url::Url;
use wreq::cookie::{CookieStore, Cookies, Jar};

/// A captured `Set-Cookie`, keyed by the host that sent it.
#[derive(Clone)]
pub struct StoredCookie {
    pub host: String,
    pub name: String,
    pub value: String,
    pub path: String,
    pub domain: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub secure: bool,
    pub http_only: bool,
}

static JAR: OnceLock<Arc<Jar>> = OnceLock::new();
static STORE: OnceLock<Arc<PersistentJar>> = OnceLock::new();
static PENDING: OnceLock<RwLock<Vec<StoredCookie>>> = OnceLock::new();

fn jar() -> Arc<Jar> {
    JAR.get_or_init(|| Arc::new(Jar::default())).clone()
}

fn pending() -> &'static RwLock<Vec<StoredCookie>> {
    PENDING.get_or_init(|| RwLock::new(Vec::new()))
}

/// Shared store handed to the HTTP clients. Delegates to the RFC-compliant jar
/// and mirrors every `Set-Cookie` for later DB persistence.
pub struct PersistentJar {
    inner: Arc<Jar>,
}

impl CookieStore for PersistentJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, uri: &Uri) {
        let host = uri.host().unwrap_or("").to_string();
        let owned: Vec<HeaderValue> = cookie_headers.cloned().collect();
        if !host.is_empty() {
            let mut list = pending().write().unwrap();
            for hv in &owned {
                if let Ok(raw) = std::str::from_utf8(hv.as_bytes()) {
                    if let Some(c) = parse_set_cookie(&host, raw) {
                        list.push(c);
                    }
                }
            }
        }
        self.inner.set_cookies(&mut owned.iter(), uri);
    }

    fn cookies(&self, uri: &Uri, version: Version) -> Cookies {
        self.inner.cookies(uri, version)
    }
}

pub fn persistent_store() -> Arc<PersistentJar> {
    STORE
        .get_or_init(|| Arc::new(PersistentJar { inner: jar() }))
        .clone()
}

fn parse_set_cookie(host: &str, raw: &str) -> Option<StoredCookie> {
    let c = cookie::Cookie::parse(raw.to_string()).ok()?;
    let expires_at = c
        .max_age()
        .map(|d| Utc::now() + chrono::Duration::seconds(d.whole_seconds()))
        .or_else(|| {
            c.expires()
                .and_then(|e| e.datetime())
                .and_then(|dt| DateTime::<Utc>::from_timestamp(dt.unix_timestamp(), 0))
        });

    Some(StoredCookie {
        host: host.to_string(),
        name: c.name().to_string(),
        value: c.value().to_string(),
        path: c.path().unwrap_or("/").to_string(),
        domain: c.domain().map(|d| d.to_string()),
        expires_at,
        secure: c.secure().unwrap_or(false),
        http_only: c.http_only().unwrap_or(false),
    })
}

#[derive(sqlx::FromRow)]
struct CookieRow {
    host: String,
    name: String,
    path: String,
    value: String,
    domain: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    secure: bool,
    http_only: bool,
}

/// Loads non-expired cookies from the DB into the in-memory jar at startup.
pub async fn load(pool: &PgPool) -> Result<(), sqlx::Error> {
    let rows: Vec<CookieRow> = sqlx::query_as(
        "SELECT host, name, path, value, domain, expires_at, secure, http_only \
         FROM domain_cookies WHERE expires_at IS NULL OR expires_at > NOW()",
    )
    .fetch_all(pool)
    .await?;

    let jar = jar();
    for CookieRow { host, name, path, value, domain, expires_at, secure, http_only } in rows {
        let mut s = format!("{}={}", name, value);
        if let Some(d) = domain {
            s.push_str(&format!("; Domain={}", d));
        }
        s.push_str(&format!("; Path={}", path));
        if let Some(exp) = expires_at {
            s.push_str(&format!("; Expires={}", exp.format("%a, %d %b %Y %H:%M:%S GMT")));
        }
        if secure {
            s.push_str("; Secure");
        }
        if http_only {
            s.push_str("; HttpOnly");
        }
        let uri = format!("https://{}{}", host, path);
        jar.add(s.as_str(), uri.as_str());
    }
    Ok(())
}

/// Persists cookies captured since the last flush and prunes expired ones.
pub async fn flush(pool: &PgPool) -> Result<(), sqlx::Error> {
    let batch: Vec<StoredCookie> = {
        let mut list = pending().write().unwrap();
        std::mem::take(&mut *list)
    };

    if batch.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    for c in &batch {
        sqlx::query(
            "INSERT INTO domain_cookies \
             (host, name, path, value, domain, expires_at, secure, http_only, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW()) \
             ON CONFLICT (host, name, path) DO UPDATE SET \
               value = EXCLUDED.value, domain = EXCLUDED.domain, \
               expires_at = EXCLUDED.expires_at, secure = EXCLUDED.secure, \
               http_only = EXCLUDED.http_only, updated_at = NOW()",
        )
        .bind(&c.host)
        .bind(&c.name)
        .bind(&c.path)
        .bind(&c.value)
        .bind(&c.domain)
        .bind(c.expires_at)
        .bind(c.secure)
        .bind(c.http_only)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("DELETE FROM domain_cookies WHERE expires_at IS NOT NULL AND expires_at < NOW()")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(())
}

/// Directory Obscura persists its browser state (cookies/localStorage) to.
/// Shared across runs so a challenge solved once is reusable.
pub fn storage_dir() -> PathBuf {
    std::env::var("OBSCURA_STORAGE_DIR")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/obscura"))
}

/// One entry of Obscura's `cookies.json` (see `docs/Persist-cookies-and-storage`).
#[derive(Deserialize)]
struct ObscuraCookie {
    name: String,
    value: String,
    domain: String,
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    secure: bool,
    #[serde(default, rename = "httpOnly")]
    http_only: bool,
    #[serde(default, rename = "sameSite")]
    same_site: String,
    #[serde(default)]
    expires: Option<i64>,
}

fn default_path() -> String {
    "/".to_string()
}

/// Imports cookies Obscura solved (typically `cf_clearance`) into the HTTP jar
/// and queues them for persistence, so the next HTTP attempt can reuse them and
/// skip the browser. Reads Obscura's `cookies.json` from the shared storage dir.
pub async fn import_from_obscura(url: &Url) {
    let host = url.host_str().unwrap_or("").to_lowercase();
    if host.is_empty() {
        return;
    }

    let path = storage_dir().join("cookies.json");
    let data = match tokio::fs::read_to_string(&path).await {
        Ok(d) => d,
        Err(_) => return,
    };
    let cookies: Vec<ObscuraCookie> = match serde_json::from_str(&data) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse Obscura cookies at {}: {}", path.display(), e);
            return;
        }
    };

    let now = Utc::now().timestamp();
    let jar = jar();
    let mut list = pending().write().unwrap();
    let mut imported = 0usize;

    for c in cookies {
        if c.expires.is_some_and(|e| e <= now) {
            continue;
        }
        let domain = c.domain.trim_start_matches('.').to_lowercase();
        if domain.is_empty() {
            continue;
        }
        // Only pull cookies that apply to the host we just fetched.
        if host != domain && !host.ends_with(&format!(".{}", domain)) {
            continue;
        }

        let mut set_cookie = format!("{}={}", c.name, c.value);
        set_cookie.push_str(&format!("; Domain={}", c.domain));
        set_cookie.push_str(&format!("; Path={}", c.path));
        if let Some(exp) = c.expires.and_then(|e| DateTime::<Utc>::from_timestamp(e, 0)) {
            set_cookie.push_str(&format!("; Expires={}", exp.format("%a, %d %b %Y %H:%M:%S GMT")));
        }
        if c.secure {
            set_cookie.push_str("; Secure");
        }
        if c.http_only {
            set_cookie.push_str("; HttpOnly");
        }
        if !c.same_site.is_empty() {
            set_cookie.push_str(&format!("; SameSite={}", c.same_site));
        }

        let uri = format!("{}://{}{}", url.scheme(), host, c.path);
        jar.add(set_cookie.as_str(), uri.as_str());

        list.push(StoredCookie {
            host: host.clone(),
            name: c.name,
            value: c.value,
            path: c.path,
            domain: Some(domain),
            expires_at: c.expires.and_then(|e| DateTime::<Utc>::from_timestamp(e, 0)),
            secure: c.secure,
            http_only: c.http_only,
        });
        imported += 1;
    }

    if imported > 0 {
        debug!("Imported {} cookie(s) from Obscura for {}", imported, host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn imports_only_cookies_for_the_fetched_host() {
        let dir = std::env::temp_dir().join(format!("purrce-obscura-cookies-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let data = serde_json::json!([
            {"name": "cf_clearance", "value": "abc", "domain": ".example.com", "path": "/",
             "secure": true, "httpOnly": true, "sameSite": "None", "expires": null},
            {"name": "unrelated", "value": "x", "domain": "other.com", "path": "/",
             "secure": false, "httpOnly": false}
        ]);
        std::fs::write(dir.join("cookies.json"), serde_json::to_string(&data).unwrap()).unwrap();
        std::env::set_var("OBSCURA_STORAGE_DIR", &dir);

        let url = Url::parse("https://www.example.com/product").unwrap();
        import_from_obscura(&url).await;

        let stored = pending().read().unwrap();
        assert!(stored.iter().any(|c| c.name == "cf_clearance" && c.value == "abc" && c.http_only));
        assert!(!stored.iter().any(|c| c.name == "unrelated"));

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("OBSCURA_STORAGE_DIR");
    }
}
