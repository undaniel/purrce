pub mod generic;
pub mod cookies;

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use rand::Rng;
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct ProductData {
    pub name: String,
    pub price: f64,
    pub currency: String,
    pub image_url: Option<String>,
    pub availability: bool,
    pub external_product_id: Option<String>,
}

impl ProductData {
    fn pending(default_currency: &str) -> Self {
        ProductData {
            name: "Pending".to_string(),
            price: 0.0,
            currency: default_currency.to_string(),
            image_url: None,
            availability: true,
            external_product_id: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("No product data found")]
    NoData,
    #[error("CAPTCHA or bot detection encountered")]
    CaptchaRequired,
    #[error("Access denied or blocked")]
    Blocked,
}

impl ExtractionError {
    /// Blocked/CAPTCHA means "try a real browser"; NoData means the page simply
    /// has no product markup and a second identical fetch will not change that.
    fn is_bot_wall(&self) -> bool {
        matches!(self, ExtractionError::CaptchaRequired | ExtractionError::Blocked)
    }
}

// --- Per-domain memory ------------------------------------------------------

#[derive(Default)]
struct DomainState {
    failures: u32,
    blocked_until: Option<Instant>,
}

// ponytail: global maps, per-domain locks if contention ever shows up
static CIRCUIT: OnceLock<Mutex<HashMap<String, DomainState>>> = OnceLock::new();

fn circuit() -> &'static Mutex<HashMap<String, DomainState>> {
    CIRCUIT.get_or_init(|| Mutex::new(HashMap::new()))
}

const CIRCUIT_THRESHOLD: u32 = 5;
const CIRCUIT_COOLDOWN: Duration = Duration::from_secs(30 * 60);

fn is_circuit_open(domain: &str) -> bool {
    let map = circuit().lock().unwrap();
    map.get(domain)
        .and_then(|s| s.blocked_until)
        .is_some_and(|until| Instant::now() < until)
}

fn record_domain_failure(domain: &str) {
    let mut map = circuit().lock().unwrap();
    let state = map.entry(domain.to_string()).or_default();
    state.failures += 1;
    if state.failures >= CIRCUIT_THRESHOLD {
        state.blocked_until = Some(Instant::now() + CIRCUIT_COOLDOWN);
        warn!(
            "Circuit breaker OPEN for {}: {} failures, pausing {}min",
            domain,
            state.failures,
            CIRCUIT_COOLDOWN.as_secs() / 60
        );
    }
}

fn record_domain_success(domain: &str) {
    circuit().lock().unwrap().remove(domain);
}

/// Domains learned to need a real browser, and those that additionally need
/// stealth. Both let later checks skip straight to the method that works.
static JS_DOMAINS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static STEALTH_DOMAINS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn domain_set(cell: &'static OnceLock<Mutex<HashSet<String>>>) -> &'static Mutex<HashSet<String>> {
    cell.get_or_init(|| Mutex::new(HashSet::new()))
}

fn in_set(cell: &'static OnceLock<Mutex<HashSet<String>>>, domain: &str) -> bool {
    domain_set(cell).lock().unwrap().contains(domain)
}

fn mark(cell: &'static OnceLock<Mutex<HashSet<String>>>, domain: &str, what: &str) {
    if domain_set(cell).lock().unwrap().insert(domain.to_string()) {
        info!("Domain {} marked as requiring {} for future requests", domain, what);
    }
}

// --- HTTP client ------------------------------------------------------------

/// Deployment profile: how much effort (and resources) a scrape may spend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScrapeProfile {
    /// Local only: no proxy, one browser attempt. Lowest resource use.
    Minimal,
    /// Honor a configured proxy and allow a stealth retry.
    Balanced,
    /// Like balanced, but every browser attempt uses stealth.
    Aggressive,
}

impl ScrapeProfile {
    pub fn from_env() -> Self {
        match std::env::var("PROFILE").unwrap_or_default().to_lowercase().as_str() {
            "minimal" => Self::Minimal,
            "aggressive" => Self::Aggressive,
            _ => Self::Balanced,
        }
    }

    fn allow_proxy(self) -> bool {
        !matches!(self, Self::Minimal)
    }

    fn max_browser_attempts(self) -> usize {
        if matches!(self, Self::Minimal) { 1 } else { 2 }
    }

    fn always_stealth(self) -> bool {
        matches!(self, Self::Aggressive)
    }

    /// How many domains the scheduler may scrape at once. Browser-heavy
    /// profiles get more headroom; minimal keeps the footprint small.
    pub fn max_concurrent_domains(self) -> usize {
        match self {
            Self::Minimal => 2,
            Self::Balanced => 4,
            Self::Aggressive => 8,
        }
    }
}

static PROFILE: OnceLock<ScrapeProfile> = OnceLock::new();

pub fn set_profile(profile: ScrapeProfile) {
    let _ = PROFILE.set(profile);
}

pub fn profile() -> ScrapeProfile {
    *PROFILE.get_or_init(ScrapeProfile::from_env)
}

/// One client per browser identity, all sharing the persistent cookie store.
/// Impersonation sets the TLS/JA3 + HTTP/2 fingerprints and coherent headers,
/// so requests are not hand-crafted header by header.
static CLIENTS: OnceLock<Vec<wreq::Client>> = OnceLock::new();

fn build_client(emulation: wreq_util::Profile, proxy: Option<String>) -> wreq::Client {
    let mut builder = wreq::Client::builder()
        .emulation(emulation)
        .cookie_provider(cookies::persistent_store())
        .gzip(true)
        .brotli(true)
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .redirect(wreq::redirect::Policy::limited(5));

    if let Some(proxy_url) = proxy {
        match wreq::Proxy::all(proxy_url.as_str()) {
            Ok(proxy) => builder = builder.proxy(proxy),
            Err(e) => warn!("Failed to configure proxy: {}", e),
        }
    }

    builder.build().expect("wreq client")
}

fn clients() -> &'static Vec<wreq::Client> {
    CLIENTS.get_or_init(|| {
        let proxy = profile().allow_proxy().then(get_proxy).flatten();
        vec![
            build_client(wreq_util::Profile::Chrome131, proxy.clone()),
            build_client(wreq_util::Profile::Firefox133, proxy.clone()),
            build_client(wreq_util::Profile::Safari16, proxy),
        ]
    })
}

fn pick_client() -> &'static wreq::Client {
    let clients = clients();
    &clients[rand::rng().random_range(0..clients.len())]
}

/// Byte-slicing a page can land mid-codepoint and panic; back off to a boundary.
fn head(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut i = max;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    &s[..i]
}

fn get_proxy() -> Option<String> {
    std::env::var("HTTP_PROXY")
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .or_else(|_| std::env::var("PROXY_URL"))
        .ok()
}

/// Caps how many Obscura processes run at once so a burst of domains cannot
/// exhaust memory. Override with `OBSCURA_MAX_CONCURRENCY` (default 3).
fn obscura_semaphore() -> &'static Semaphore {
    static SEM: OnceLock<Semaphore> = OnceLock::new();
    SEM.get_or_init(|| {
        let n = std::env::var("OBSCURA_MAX_CONCURRENCY")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(3);
        Semaphore::new(n)
    })
}

// --- Main extraction --------------------------------------------------------

/// `db_needs_js` / `db_needs_stealth`: persisted hints loaded from the stores table.
/// When true they pre-seed the in-memory sets so we skip the HTTP attempt on restart.
pub async fn extract_product(
    url: &Url,
    default_currency: &str,
    db_needs_js: bool,
    db_needs_stealth: bool,
) -> Result<(ProductData, bool, bool), ExtractionError> {
    let domain = url.host_str().unwrap_or("unknown").to_string();

    // Pre-seed from DB so we don't retry plain HTTP on every restart
    if db_needs_js || always_needs_js(&domain) {
        mark(&JS_DOMAINS, &domain, "JavaScript");
    }
    if db_needs_stealth || always_needs_stealth(&domain) {
        mark(&STEALTH_DOMAINS, &domain, "stealth");
    }

    if is_circuit_open(&domain) {
        info!("Circuit breaker open for {}, skipping fetch", domain);
        return Ok((ProductData::pending(default_currency), false, false));
    }

    let needs_js = in_set(&JS_DOMAINS, &domain);
    let needs_stealth = in_set(&STEALTH_DOMAINS, &domain);

    // 1. Chrome headers, then 2. Firefox headers. These are plain `reqwest`
    //    requests: coherent headers and a cookie jar, but NOT a spoofed TLS/HTTP2
    //    fingerprint, so a serious bot wall can still tell it apart. They are
    //    ~10x cheaper than a browser, so they run first and Obscura handles the rest.
    //    (For true impersonation, swap in a fingerprinting client such as rquest.)
    let mut http_hit_bot_wall = false;
    if !needs_js {
        let t = Instant::now();
        match try_http_with_retry(url, default_currency).await {
            Ok(data) if is_valid_product(&data) => {
                info!(url = url.as_str(), elapsed_ms = t.elapsed().as_millis(), price = data.price, "scraper: http success");
                record_domain_success(&domain);
                return Ok((data, false, false));
            }
            Ok(_) => debug!(url = url.as_str(), "scraper: http gave no usable product, escalating"),
            Err(e) => {
                http_hit_bot_wall = e.is_bot_wall();
                debug!(url = url.as_str(), error = %e, "scraper: http failed, escalating");
            }
        }
        debug!(elapsed_ms = t.elapsed().as_millis(), "scraper: http phase done");
    }

    // 2. Headless browser. Go straight to stealth when we already know this
    //    domain needs it, or when plain HTTP was actively blocked - the extra
    //    non-stealth pass would just burn another browser launch.
    //    First attempt uses `load` (fast: ~3-5s); stealth attempt uses
    //    `networkidle2` (thorough: waits for late-rendered prices).
    let start_stealth = needs_stealth || http_hit_bot_wall || profile().always_stealth();
    let max_attempts = profile().max_browser_attempts();
    let mut last_err = None;

    // A stealth-first domain only needs the strongest attempt; otherwise try
    // `load` then escalate to stealth. `max_attempts` lets minimal mode stop early.
    let plan: &[bool] = if start_stealth { &[true] } else { &[false, true] };
    for &stealth in plan.iter().take(max_attempts) {
        let wait = if stealth { "networkidle2" } else { "load" };
        let t = Instant::now();
        match try_obscura_extract(url, stealth, wait, default_currency).await {
            Ok(data) if is_valid_product(&data) => {
                info!(url = url.as_str(), stealth, elapsed_ms = t.elapsed().as_millis(), price = data.price, "scraper: browser success");
                record_domain_success(&domain);
                mark(&JS_DOMAINS, &domain, "JavaScript");
                if stealth {
                    mark(&STEALTH_DOMAINS, &domain, "stealth");
                }
                return Ok((data, true, stealth));
            }
            Ok(_) => {
                // The page rendered but carries no product markup. A stealth
                // retry renders the same DOM, so stop here.
                info!(url = url.as_str(), stealth, elapsed_ms = t.elapsed().as_millis(), "scraper: rendered page has no product data");
                record_domain_failure(&domain);
                return Ok((ProductData::pending(default_currency), true, stealth));
            }
            Err(e) => {
                debug!(url = url.as_str(), stealth, elapsed_ms = t.elapsed().as_millis(), error = %e, "scraper: browser attempt failed");
                record_domain_failure(&domain);
                last_err = Some(e);
            }
        }
    }

    info!(url = url.as_str(), error = ?last_err.map(|e| e.to_string()), "scraper: all methods failed, returning Pending");
    Ok((ProductData::pending(default_currency), false, false))
}

fn is_valid_product(data: &ProductData) -> bool {
    let invalid_names = [
        "page not found",
        "error page",
        "access denied",
        "página en construcción",
        "just a moment",
        "404 not found",
        "403 forbidden",
        "captcha",
        "blocked",
    ];

    let name_lower = data.name.to_lowercase();

    if invalid_names.iter().any(|invalid| name_lower.contains(invalid)) {
        return false;
    }
    if data.name.trim().len() < 3 || data.price <= 0.0 {
        return false;
    }

    // Currencies without cents in practice: a sub-100 value is a parse error.
    if data.price < 100.0 && matches!(data.currency.as_str(), "CLP" | "ARS" | "COP") {
        return false;
    }

    true
}

/// Detects a shell page whose product data only appears after JavaScript runs.
fn needs_javascript(html: &str) -> bool {
    // Cheap structural check first; a full page can be megabytes.
    let head = head(html, 80_000);
    let lower = head.to_lowercase();

    // Only explicit "you need JS" signals — counting <script> tags was a false
    // positive: Next/Nuxt sites SSR the price and still ship many <script> tags.
    let indicators = [
        "please enable javascript",
        "necesita javascript",
        "javascript is required",
        "enable javascript to continue",
        "window.__initial_state__",
        "window.__data__",
    ];

    indicators.iter().any(|i| lower.contains(i))
}

// ponytail: one quick retry for transient network errors only; bot walls and
// missing markup are answered by escalating to the browser, not by repeating.
async fn try_http_with_retry(url: &Url, default_currency: &str) -> Result<ProductData, ExtractionError> {
    match try_http_extract(url, default_currency).await {
        Err(ExtractionError::Http(e)) => {
            debug!("http attempt failed: {}, one retry in 300ms", e);
            tokio::time::sleep(Duration::from_millis(300)).await;
            try_http_extract(url, default_currency).await
        }
        other => other,
    }
}

async fn try_http_extract(url: &Url, default_currency: &str) -> Result<ProductData, ExtractionError> {
    let html = fetch_html(url).await?;
    debug!("HTML received ({} bytes)", html.len());

    if is_captcha_or_blocked(&html) {
        warn!("CAPTCHA/bot detection triggered for {}", url);
        return Err(ExtractionError::CaptchaRequired);
    }
    if needs_javascript(&html) {
        debug!("Page detected as JS-heavy, skipping http extraction");
        return Err(ExtractionError::NoData);
    }

    let doc = scraper::Html::parse_document(&html);
    generic::extract(&doc, url, default_currency).ok_or(ExtractionError::NoData)
}

async fn try_obscura_extract(url: &Url, stealth: bool, wait_until: &str, default_currency: &str) -> Result<ProductData, ExtractionError> {
    use tokio::process::Command;

    // Bound concurrent browsers: a burst of domains must not blow up memory.
    let _permit = obscura_semaphore()
        .acquire()
        .await
        .expect("obscura semaphore never closes");

    let obscura_bin = std::env::var("OBSCURA_BIN").unwrap_or_else(|_| "obscura".to_string());
    let timeout = std::env::var("OBSCURA_TIMEOUT").unwrap_or_else(|_| "20".to_string());
    // OBSCURA_WAIT overrides the per-attempt strategy (load vs networkidle2)
    let effective_wait = std::env::var("OBSCURA_WAIT").unwrap_or_else(|_| wait_until.to_string());
    // Keep V8 on a short leash: heavy SPAs otherwise grow without bound.
    let v8_flags = std::env::var("OBSCURA_V8_FLAGS")
        .unwrap_or_else(|_| "--max-old-space-size=256".to_string());
    // Cap the page's script-execution phase so one hung SPA cannot stall a worker.
    let script_deadline = std::env::var("OBSCURA_SCRIPT_DEADLINE_MS")
        .unwrap_or_else(|_| "20000".to_string());
    let proxy = get_proxy();
    // Persist the browser profile so challenges solved once (cf_clearance, etc.)
    // survive across runs and can be imported into the HTTP jar afterwards.
    let storage_dir = cookies::storage_dir().to_string_lossy().to_string();

    // `--v8-flags` is a global flag, so it must precede the `fetch` subcommand.
    let mut args = vec![
        "--v8-flags",
        &v8_flags,
        "fetch",
        url.as_str(),
        "--dump",
        "html",
        "--wait-until",
        &effective_wait,
        "--timeout",
        &timeout,
        "--storage-dir",
        &storage_dir,
        "--quiet",
    ];

    if stealth {
        args.push("--stealth");
    }
    if let Some(ref proxy_url) = proxy {
        args.push("--proxy");
        args.push(proxy_url);
    }

    debug!(stealth, "Running obscura fetch for: {}", url);

    let output = Command::new(&obscura_bin)
        .args(&args)
        .env("OBSCURA_SCRIPT_DEADLINE_MS", &script_deadline)
        .output()
        .await
        .map_err(|e| ExtractionError::Http(format!("Failed to execute obscura: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Obscura failed: {}", stderr);
        return Err(ExtractionError::Http(format!("Obscura failed: {}", stderr)));
    }

    let html = String::from_utf8_lossy(&output.stdout);
    debug!("Obscura HTML received ({} bytes)", html.len());

    // Share whatever the browser obtained with the HTTP rung.
    cookies::import_from_obscura(url).await;

    if html.is_empty() {
        return Err(ExtractionError::NoData);
    }
    if is_captcha_or_blocked(&html) {
        warn!("CAPTCHA/bot detection in Obscura response for {}", url);
        return Err(ExtractionError::CaptchaRequired);
    }

    let doc = scraper::Html::parse_document(&html);
    match generic::extract(&doc, url, default_currency) {
        Some(data) => Ok(data),
        None => generic::extract_fallback(&doc, url, default_currency),
    }
}

/// Domains that consistently require a real browser — pre-seeded so we skip the
/// pointless HTTP attempt on first request and go straight to Obscura.
static ALWAYS_JS_DOMAINS: &[&str] = &[
    // eBay (global)
    "ebay.com", "ebay.es", "ebay.co.uk", "ebay.de", "ebay.fr", "ebay.it", "ebay.com.au",
    // Amazon (prices only render after JS hydration)
    "amazon.com", "amazon.es", "amazon.com.mx", "amazon.com.br",
    "amazon.co.uk", "amazon.de", "amazon.fr", "amazon.it",
    // Chilean / LatAm retailers (React/Next SPA shells)
    "falabella.com",
    "paris.cl",
    "ripley.cl",
    "mercadolibre.cl", "mercadolibre.com", "mercadolibre.com.ar",
    "mercadolibre.com.mx", "mercadolibre.com.co",
    "lider.cl",
    "jumbo.cl",
    "easy.cl",
    "corona.cl",
    "sodimac.cl",
    "pcfactory.cl",
    "abcdin.cl",
    "walmart.cl",
    "casaideas.cl",
];

/// Domains known to have Cloudflare or aggressive bot detection that blocks
/// plain headless Chromium — go straight to stealth mode, skip the fast pass.
static ALWAYS_STEALTH_DOMAINS: &[&str] = &[
    "falabella.com",
    "paris.cl",
    "ripley.cl",
    "lider.cl",
    "walmart.cl",
];

fn always_needs_js(domain: &str) -> bool {
    ALWAYS_JS_DOMAINS.iter().any(|d| domain == *d || domain.ends_with(&format!(".{}", d)))
}

fn always_needs_stealth(domain: &str) -> bool {
    ALWAYS_STEALTH_DOMAINS.iter().any(|d| domain == *d || domain.ends_with(&format!(".{}", d)))
}

fn is_captcha_or_blocked(html: &str) -> bool {
    let lower = head(html, 60_000).to_lowercase();

    // Structural signals are unambiguous and always mean a bot wall.
    if lower.contains("cf-challenge") || lower.contains("cloudflare ray id") {
        return true;
    }

    // Textual signals can also show up in a legit error toast or a help article,
    // so only trust them on a small page — bot walls are short, product pages
    // are large. On big pages, require the strongest phrases.
    let textual = [
        "captcha challenge",
        "recaptcha api",
        "hcaptcha.com",
        "please complete the security check",
        "verify you are human",
        "access denied",
        "your request has been blocked",
        "detected unusual traffic",
        // eBay / Akamai
        "security measure",
        "help us protect your account",
        "_sec_cpt",
        "ak_bmsc",
    ];

    if html.len() < 150_000 {
        return textual.iter().any(|i| lower.contains(i));
    }
    [
        "captcha challenge",
        "verify you are human",
        "please complete the security check",
    ]
    .iter()
    .any(|i| lower.contains(i))
}

/// Two attempts with different impersonated browser identities. The client
/// already sends a coherent fingerprint + header set, so no headers are
/// hand-crafted here. A bot wall on the first attempt falls through to the
/// second with a different identity.
async fn fetch_html(url: &Url) -> Result<String, ExtractionError> {
    let mut last = None;
    for attempt in 0..2 {
        let client = pick_client();
        let t = Instant::now();
        match client.get(url.as_str()).send().await {
            Ok(resp) => match check_and_read(resp).await {
                Ok(html) if !is_captcha_or_blocked(&html) => {
                    debug!(url = url.as_str(), attempt, elapsed_ms = t.elapsed().as_millis(), "http fetch ok");
                    return Ok(html);
                }
                Ok(_) => {
                    debug!(url = url.as_str(), attempt, "http fetch hit a bot wall");
                    last = Some(ExtractionError::CaptchaRequired);
                }
                Err(e) => {
                    debug!(url = url.as_str(), attempt, error = %e, "http fetch failed");
                    last = Some(e);
                }
            },
            Err(e) => last = Some(ExtractionError::Http(e.to_string())),
        }
    }
    Err(last.unwrap_or(ExtractionError::NoData))
}

async fn check_and_read(resp: wreq::Response) -> Result<String, ExtractionError> {
    let code = resp.status().as_u16();
    if code == 403 || code == 429 {
        return Err(ExtractionError::Blocked);
    }
    if !resp.status().is_success() {
        return Err(ExtractionError::Http(format!("status {}", resp.status())));
    }
    resp.text().await.map_err(|e| ExtractionError::Http(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_unit_prices_rejected_only_for_centless_currencies() {
        let mut d = ProductData::pending("CLP");
        d.name = "Mouse gamer".into();
        d.price = 19.99;
        assert!(!is_valid_product(&d), "19.99 CLP is a parse error");
        d.currency = "USD".into();
        assert!(is_valid_product(&d), "19.99 USD is a real price");
    }
}
