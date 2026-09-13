use std::sync::OnceLock;

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use super::ProductData;

/// Caches a parsed `Selector` per call site. Re-parsing CSS on every extraction
/// was a measurable chunk of the scrape budget on big pages.
macro_rules! sel {
    ($s:expr) => {{
        static S: OnceLock<Selector> = OnceLock::new();
        S.get_or_init(|| Selector::parse($s).expect("static selector"))
    }};
}

macro_rules! rex {
    ($s:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($s).expect("static regex"))
    }};
}

/// Body text is expensive on large pages, so it is built at most once per
/// extraction and shared by every step that needs it.
struct Page<'a> {
    doc: &'a Html,
    url: &'a Url,
    default_currency: &'a str,
    text: OnceLock<String>,
}

impl<'a> Page<'a> {
    fn new(doc: &'a Html, url: &'a Url, default_currency: &'a str) -> Self {
        Page { doc, url, default_currency, text: OnceLock::new() }
    }

    fn text(&self) -> &str {
        self.text.get_or_init(|| {
            // Only <body>: <head> holds script/style text, which is pure noise.
            let mut s = String::new();
            if let Some(body) = self.doc.select(sel!("body")).next() {
                for chunk in body.text() {
                    s.push_str(chunk);
                    s.push(' ');
                    if s.len() > 200_000 {
                        break; // ponytail: prices live near the top; cap the scan
                    }
                }
            } else {
                s = self.doc.root_element().text().collect::<String>();
            }
            s
        })
    }

    fn currency(&self) -> String {
        detect_currency(self.text(), self.url, self.default_currency)
    }
}

/// Detects currency from text content and URL domain.
/// Priority: code adjacent to digits > symbol > domain > loose code scan > fallback
pub fn detect_currency(text: &str, url: &Url, fallback: &str) -> String {
    // Scan only the head of the text: currency markers sit next to the price.
    let head = super::head(text, 20_000);
    let text_upper = head.to_uppercase();

    // "US$" / "US $" must be checked before scanning for bare "USD" or "$".
    if head.contains("US$") || head.contains("US $") {
        return "USD".to_string();
    }
    if head.contains("R$") {
        return "BRL".to_string();
    }

    const CODES: &[&str] = &["USD", "EUR", "GBP", "BRL", "ARS", "MXN", "CLP", "COP", "PEN", "UYU"];

    // Pass 1: code immediately adjacent to a digit — "CLP 1,494" beats a stray "EUR" in specs.
    // ponytail: simple byte scan, no regex needed
    for &code in CODES {
        let mut search = text_upper.as_str();
        while let Some(pos) = search.find(code) {
            let after = search[pos + code.len()..].trim_start_matches(' ');
            if after.starts_with(|c: char| c.is_ascii_digit()) {
                return code.to_string();
            }
            search = &search[pos + 1..];
        }
    }

    if head.contains('\u{20ac}') {
        return "EUR".to_string();
    }
    if head.contains('\u{a3}') {
        return "GBP".to_string();
    }

    // Pass 2: the store's domain is a stronger signal than a loose code scan.
    // Currency selectors and metadata routinely mention several currencies
    // (e.g. a Chilean shop listing "ARS" in a switcher), so a bare code match
    // must not override a known market TLD.
    let host_lower = url.host_str().unwrap_or("").to_lowercase();
    if let Some(c) = currency_for_host(&host_lower) {
        return c;
    }

    // Pass 3: loose code anywhere in head, as a whole word only.
    for &code in CODES {
        if contains_word(&text_upper, code) {
            return code.to_string();
        }
    }

    if head.contains('$') && !host_lower.is_empty() {
        return "USD".to_string();
    }

    fallback.to_string()
}

/// Whether `code` appears in `text` delimited by non-alphanumeric characters,
/// so "ARS" does not match inside "GUITARS" or "VERSIÓN".
fn contains_word(text: &str, code: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(pos) = text[start..].find(code) {
        let i = start + pos;
        let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        let end = i + code.len();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = i + 1;
    }
    false
}

fn currency_for_host(host: &str) -> Option<String> {
    let c = if host.ends_with(".cl") || host.contains("mercadolibre.cl") {
        "CLP"
    } else if host.ends_with(".ar") || host.contains("mercadolibre.com.ar") {
        "ARS"
    } else if host.ends_with(".mx") || host.contains("mercadolibre.com.mx") {
        "MXN"
    } else if host.ends_with(".br") {
        "BRL"
    } else if host.ends_with(".uk") {
        "GBP"
    } else if host.ends_with(".pe") {
        "PEN"
    } else if host.ends_with(".de")
        || host.ends_with(".fr")
        || host.ends_with(".es")
        || host.ends_with(".it")
    {
        "EUR"
    } else {
        return None;
    };
    Some(c.to_string())
}

/// Currencies with no cents in practice: a two-digit "price" there is a parse error.
fn min_plausible_price(currency: &str) -> f64 {
    match currency {
        "CLP" | "ARS" | "COP" | "PEN" | "UYU" => 100.0,
        _ => 0.5,
    }
}

// --- Public entry point -----------------------------------------------------

/// Runs every extractor and merges the results instead of trusting whichever one
/// answered first. A page whose JSON-LD carries the name but no price used to
/// fall straight through to the (slow) browser path; now OpenGraph or the DOM
/// fills the gap.
pub fn extract(doc: &Html, url: &Url, default_currency: &str) -> Option<ProductData> {
    let page = Page::new(doc, url, default_currency);

    let mut merged: Option<ProductData> = None;
    for candidate in [
        extract_jsonld_page(&page),
        extract_opengraph_page(&page),
        extract_meta_page(&page),
        extract_from_html_page(&page),
    ]
    .into_iter()
    .flatten()
    {
        match merged {
            None => merged = Some(candidate),
            Some(ref mut m) => {
                if m.price <= 0.0 && candidate.price > 0.0 {
                    m.price = candidate.price;
                    m.currency = candidate.currency;
                }
                if m.image_url.is_none() {
                    m.image_url = candidate.image_url;
                }
                if m.external_product_id.is_none() {
                    m.external_product_id = candidate.external_product_id;
                }
                if m.name.len() < 3 {
                    m.name = candidate.name;
                }
            }
        }
    }

    if let Some(ref mut m) = merged {
        m.name = clean_name(&m.name);
    }
    merged
}

/// Decodes HTML entities and collapses whitespace. Needed because some sites
/// emit `&#8211;` inside JSON-LD, where the `<script>` body is raw text and is
/// never entity-decoded by the HTML parser.
fn clean_name(raw: &str) -> String {
    normalize_ws(&html_escape::decode_html_entities(raw))
}

/// Title-only result, used when nothing structured was found.
pub fn extract_fallback(
    doc: &Html,
    url: &Url,
    default_currency: &str,
) -> Result<ProductData, super::ExtractionError> {
    let page = Page::new(doc, url, default_currency);
    let name = extract_title(doc).unwrap_or_else(|| {
        url.path_segments()
            .and_then(|mut s| s.next_back())
            .unwrap_or("Unknown")
            .to_string()
    });

    let name = clean_name(&name);
    Ok(ProductData {
        name: if name.is_empty() { "Unknown".to_string() } else { name },
        price: 0.0,
        currency: page.currency(),
        image_url: None,
        availability: true,
        external_product_id: None,
    })
}

// --- Extractors -------------------------------------------------------------

fn extract_jsonld_page(page: &Page) -> Option<ProductData> {
    for script in page.doc.select(sel!(r#"script[type="application/ld+json"]"#)) {
        let text = script.text().collect::<String>();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let Ok(val) = serde_json::from_str::<serde_json::Value>(text) else {
            continue;
        };
        if let Some(data) = find_jsonld_product(&val, page, 0) {
            return Some(data);
        }
    }
    None
}

/// Walks arrays and `@graph` containers, which Shopify/WooCommerce/Next sites
/// emit far more often than a bare top-level Product object.
fn find_jsonld_product(val: &serde_json::Value, page: &Page, depth: u8) -> Option<ProductData> {
    if depth > 4 {
        return None;
    }
    if let Some(arr) = val.as_array() {
        return arr.iter().find_map(|v| find_jsonld_product(v, page, depth + 1));
    }
    if let Some(data) = parse_jsonld_product(val, page) {
        return Some(data);
    }
    for key in ["@graph", "mainEntity", "itemListElement"] {
        if let Some(child) = val.get(key) {
            if let Some(data) = find_jsonld_product(child, page, depth + 1) {
                return Some(data);
            }
        }
    }
    None
}

fn jsonld_is_product(val: &serde_json::Value) -> bool {
    let matches = |s: &str| {
        let s = s.rsplit('/').next().unwrap_or(s);
        s.eq_ignore_ascii_case("Product") || s.eq_ignore_ascii_case("ProductModel")
    };
    match val.get("@type") {
        Some(serde_json::Value::String(s)) => matches(s),
        Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).any(matches),
        _ => false,
    }
}

fn parse_jsonld_product(val: &serde_json::Value, page: &Page) -> Option<ProductData> {
    if !jsonld_is_product(val) {
        return None;
    }
    let name = val.get("name")?.as_str()?.trim().to_string();

    let image_url = val.get("image").and_then(jsonld_image);

    let (price, currency, availability) = val
        .get("offers")
        .and_then(|o| jsonld_offer(o, 0))
        .unwrap_or((0.0, None, true));

    let external_product_id = ["sku", "mpn", "gtin13", "productID"]
        .iter()
        .find_map(|k| val.get(*k))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(ProductData {
        name,
        price,
        currency: currency.unwrap_or_else(|| page.currency()),
        image_url,
        availability,
        external_product_id,
    })
}

fn jsonld_image(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(a) => a.iter().find_map(jsonld_image),
        serde_json::Value::Object(o) => o.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
        _ => None,
    }
}

/// Returns (price, currency, availability) from an `offers` node, following
/// arrays, AggregateOffer and priceSpecification.
fn jsonld_offer(offer: &serde_json::Value, depth: u8) -> Option<(f64, Option<String>, bool)> {
    if depth > 3 {
        return None;
    }
    if let Some(arr) = offer.as_array() {
        // Prefer an in-stock offer with a real price, else any priced offer.
        let mut fallback = None;
        for item in arr {
            if let Some(found) = jsonld_offer(item, depth + 1) {
                if found.0 > 0.0 && found.2 {
                    return Some(found);
                }
                if fallback.is_none() {
                    fallback = Some(found);
                }
            }
        }
        return fallback;
    }

    let price = ["price", "lowPrice", "highPrice"]
        .iter()
        .find_map(|k| offer.get(*k))
        .and_then(json_number)
        .or_else(|| {
            offer
                .get("priceSpecification")
                .and_then(|s| jsonld_offer(s, depth + 1))
                .map(|(p, _, _)| p)
        })
        .unwrap_or(0.0);

    let currency = offer
        .get("priceCurrency")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_uppercase())
        .filter(|s| s.len() == 3);

    let availability = offer
        .get("availability")
        .and_then(|v| v.as_str())
        .map(|s| {
            let s = s.to_lowercase();
            !(s.contains("outofstock") || s.contains("soldout") || s.contains("discontinued"))
        })
        .unwrap_or(true);

    if price <= 0.0 && currency.is_none() {
        return None;
    }
    Some((price, currency, availability))
}

fn json_number(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| parse_price(v.as_str()?))
}

fn extract_opengraph_page(page: &Page) -> Option<ProductData> {
    let mut title = None;
    let mut price: Option<f64> = None;
    let mut currency = None;
    let mut image: Option<String> = None;
    let mut availability = None;

    for meta in page.doc.select(sel!("meta[property], meta[name]")) {
        let el = meta.value();
        let Some(prop) = el.attr("property").or_else(|| el.attr("name")) else {
            continue;
        };
        let Some(content) = el.attr("content") else { continue };
        if content.is_empty() {
            continue;
        }
        match prop {
            "og:title" => title = Some(content.trim().to_string()),
            // product:price:* is what Shopify / Facebook catalogs actually emit.
            "og:price:amount" | "product:price:amount" => {
                price = price.or_else(|| parse_price(content))
            }
            "og:price:currency" | "product:price:currency" => {
                currency = Some(content.trim().to_uppercase())
            }
            "og:image" | "og:image:secure_url" => {
                image = image.or_else(|| Some(content.to_string()))
            }
            "og:availability" | "product:availability" => {
                let c = content.to_lowercase();
                availability =
                    Some(!(c.contains("outofstock") || c.contains("out of stock") || c.contains("oos")));
            }
            _ => {}
        }
    }

    Some(ProductData {
        name: title?,
        price: price.unwrap_or(0.0),
        currency: currency.filter(|c| c.len() == 3).unwrap_or_else(|| page.currency()),
        image_url: image,
        availability: availability.unwrap_or(true),
        external_product_id: None,
    })
}

fn extract_meta_page(page: &Page) -> Option<ProductData> {
    let mut name: Option<String> = None;
    let mut price: Option<f64> = None;
    let mut image: Option<String> = None;
    let mut currency_hint = None;

    for meta in page.doc.select(sel!("meta[itemprop]")) {
        let el = meta.value();
        let content = el.attr("content").unwrap_or("");
        if content.is_empty() {
            continue;
        }
        match el.attr("itemprop").unwrap_or("") {
            "name" => name = name.or_else(|| Some(content.trim().to_string())),
            "price" => price = price.or_else(|| parse_price(content)),
            "image" => image = image.or_else(|| Some(content.to_string())),
            "priceCurrency" => currency_hint = Some(content.trim().to_uppercase()),
            _ => {}
        }
    }

    Some(ProductData {
        name: name?,
        price: price.unwrap_or(0.0),
        currency: currency_hint.filter(|c| c.len() == 3).unwrap_or_else(|| page.currency()),
        image_url: image,
        availability: true,
        external_product_id: None,
    })
}

/// Domain-specific CSS selectors tried before the generic list. Avoids scanning
/// 7 generic selectors on sites where the exact class is known.
fn domain_price_selector(host: &str) -> Option<&'static str> {
    let host = host.to_lowercase();
    if host.contains("mercadolibre") {
        Some(".andes-money-amount__fraction")
    } else if host.contains("falabella") {
        Some("[data-internet-price], .jsx-price-box, .product-price")
    } else if host.contains("paris") {
        Some(".product-price, [data-price]")
    } else if host.contains("ripley") {
        Some(".price, .product-price, [data-price]")
    } else if host.contains("amazon") {
        Some(".a-price-whole, #priceblock_ourprice, #priceblock_dealprice, .apexPriceToPay")
    } else if host.contains("ebay") {
        Some(".x-price-primary, .x-bin-price__content")
    } else if host.contains("lider") || host.contains("walmart") {
        Some("[data-testid='product-price'], .product-price")
    } else if host.contains("sodimac") || host.contains("easy") || host.contains("corona") {
        Some(".price-box__price, .product-price, [data-price]")
    } else {
        None
    }
}

fn extract_from_html_page(page: &Page) -> Option<ProductData> {
    let name = extract_title(page.doc)?;
    if name.is_empty() {
        return None;
    }
    let (price, currency) = extract_price_from_text(page);

    Some(ProductData {
        name,
        price,
        currency,
        image_url: extract_main_image(page.doc, page.url),
        availability: true,
        external_product_id: None,
    })
}

fn extract_title(doc: &Html) -> Option<String> {
    if let Some(el) = doc.select(sel!(r#"meta[property="og:title"]"#)).next() {
        if let Some(content) = el.value().attr("content") {
            if !content.trim().is_empty() {
                return Some(content.trim().to_string());
            }
        }
    }

    if let Some(el) = doc.select(sel!("h1")).next() {
        let text = normalize_ws(&el.text().collect::<String>());
        if text.len() > 3 {
            return Some(text);
        }
    }

    if let Some(el) = doc.select(sel!("title")).next() {
        let text = normalize_ws(&el.text().collect::<String>());
        if !text.is_empty() {
            // Strip a "… | Store" suffix generically instead of per-retailer.
            let cleaned = text
                .rsplit_once(" | ")
                .or_else(|| text.rsplit_once(" - "))
                .map(|(head, _)| head.trim())
                .filter(|head| head.len() >= 10)
                .unwrap_or(text.as_str());
            return Some(cleaned.to_string());
        }
    }

    None
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Returns (price, currency). Prefers structured price nodes; among several
/// candidates it takes the lowest, which is the sale price on pages that also
/// show a crossed-out list price.
fn extract_price_from_text(page: &Page) -> (f64, String) {
    let currency = page.currency();
    let floor = min_plausible_price(&currency);

    // Try a domain-specific selector first — faster and more precise than scanning
    // the generic list on known retailers.
    if let Some(css) = page.url.host_str().and_then(domain_price_selector) {
        if let Ok(s) = scraper::Selector::parse(css) {
            for el in page.doc.select(&s).take(5) {
                let v = el.value();
                let from_attr = ["content", "data-price", "data-internet-price", "data-price-amount"]
                    .iter()
                    .find_map(|a| v.attr(a))
                    .and_then(parse_price);
                let p = from_attr.or_else(|| {
                    let text = el.text().collect::<String>();
                    if text.contains('%') { return None; }
                    parse_price(&text)
                });
                if let Some(p) = p {
                    if p >= floor {
                        return (p, currency);
                    }
                }
            }
        }
    }

    // Ordered by trustworthiness; the first selector that yields anything wins.
    let selectors: [&Selector; 7] = [
        sel!("[itemprop='price']"),
        sel!("[data-price], [data-product-price], [data-price-amount]"),
        // eBay: primary price container
        sel!(".x-price-primary, .x-bin-price__content, .x-buybox__price"),
        sel!(".price-now, .sale-price, .product-price, .price__current, .current-price"),
        sel!(".price, .precio"),
        sel!("[class*='price'], [class*='Price']"),
        sel!("[class*='precio'], [class*='Precio']"),
    ];

    // Words that indicate a non-price number (stock count, qty, rating, etc.)
    let noise_words = ["stock", "unidad", "qty", "quantity", "rating", "review", "sold", "vendido", "disponible"];

    for s in selectors {
        let mut best: Option<f64> = None;
        for el in page.doc.select(s).take(40) {
            let v = el.value();
            let from_attr = ["content", "data-price", "data-product-price", "data-price-amount"]
                .iter()
                .find_map(|a| v.attr(a))
                .and_then(parse_price);
            // Defer text collection: most structured nodes carry the price in an
            // attribute, so we avoid the String allocation in the common case.
            let candidate = from_attr.or_else(|| {
                let text = el.text().collect::<String>();
                if text.contains('%') {
                    return None;
                }
                let text_lower = text.to_lowercase();
                if noise_words.iter().any(|w| text_lower.contains(w)) {
                    return None;
                }
                parse_price(&text)
            });
            if let Some(p) = candidate {
                // When multiple prices exist on the same selector (list vs sale),
                // the lower one is the sale/current price.
                if p >= floor && best.is_none_or(|prev| p < prev) {
                    best = Some(p);
                }
            }
        }
        if let Some(p) = best {
            return (p, currency);
        }
    }

    // ponytail: last resort, largest number on the page. Wrong often enough that
    // is_valid_product still gets to veto it.
    let body = page.text();
    let mut best = 0.0f64;
    for caps in rex!(r"[$\u{20ac}\u{a3}]\s*([\d][\d.,]{0,15})").captures_iter(super::head(body, 60_000)) {
        if let Some(p) = caps.get(1).and_then(|m| parse_price(m.as_str())) {
            if p > best && p >= floor {
                best = p;
            }
        }
    }
    (best, currency)
}

fn extract_main_image(doc: &Html, url: &Url) -> Option<String> {
    if let Some(el) = doc.select(sel!(r#"meta[property="og:image"]"#)).next() {
        if let Some(content) = el.value().attr("content") {
            if !content.is_empty() {
                return Some(content.to_string());
            }
        }
    }

    if let Some(el) = doc
        .select(sel!(r#"img[itemprop="image"], img.product-image, img.main-image"#))
        .next()
    {
        if let Some(src) = el.value().attr("src").or_else(|| el.value().attr("data-src")) {
            return Some(resolve_url(url, src));
        }
    }

    for el in doc.select(sel!("img")).take(60) {
        if let Some(src) = el.value().attr("src").or_else(|| el.value().attr("data-src")) {
            if src.contains("product") || src.contains("media") || src.contains("image") {
                return Some(resolve_url(url, src));
            }
        }
    }

    None
}

fn resolve_url(base: &Url, path: &str) -> String {
    if path.starts_with("http") {
        return path.to_string();
    }
    base.join(path).map(|u| u.to_string()).unwrap_or_else(|_| path.to_string())
}

/// Parses a price out of arbitrary text. The separator's role is decided by how
/// many digits follow the LAST separator: 3 means thousands (Chilean "1.990"),
/// 1-2 means decimals ("19.99", "1234,56").
pub fn parse_price(s: &str) -> Option<f64> {
    // Grab the first number-ish run so surrounding words never reach the parser.
    let m = rex!(r"\d[\d.,\u{00a0} ]*\d|\d").find(s)?;
    let raw: String = m
        .as_str()
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let raw = raw.trim_matches(|c| c == '.' || c == ',');
    if raw.is_empty() {
        return None;
    }

    let normalized = match raw.rfind(['.', ',']) {
        None => raw.to_string(),
        Some(i) => {
            let decimals = raw.len() - i - 1;
            if decimals == 1 || decimals == 2 {
                let (head, tail) = raw.split_at(i);
                format!("{}.{}", head.replace(['.', ','], ""), &tail[1..])
            } else {
                raw.replace(['.', ','], "")
            }
        }
    };

    let v = normalized.parse::<f64>().ok()?;
    if v.is_finite() && v > 0.0 {
        Some(v)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_parse_by_separator_shape() {
        assert_eq!(parse_price("$1.990"), Some(1990.0)); // CLP thousands
        assert_eq!(parse_price("$1.234.567"), Some(1234567.0));
        assert_eq!(parse_price("US$ 19.99"), Some(19.99));
        assert_eq!(parse_price("1,234.56"), Some(1234.56));
        assert_eq!(parse_price("1.234,56"), Some(1234.56));
        assert_eq!(parse_price("1234,56"), Some(1234.56));
        assert_eq!(parse_price("Precio: 45.900 pesos"), Some(45900.0));
        assert_eq!(parse_price("sin precio"), None);
        assert_eq!(parse_price(""), None);
    }

    #[test]
    fn jsonld_reads_graph_and_offer_arrays() {
        let url = Url::parse("https://tienda.cl/p/1").unwrap();
        let html = Html::parse_document(
            r#"<html><head><script type="application/ld+json">
            {"@graph":[{"@type":"WebPage"},{"@type":["Product"],"name":"Teclado",
            "sku":"K1","image":{"url":"https://x/i.jpg"},
            "offers":[{"@type":"Offer","price":"29990","priceCurrency":"CLP",
            "availability":"https://schema.org/InStock"}]}]}
            </script></head><body></body></html>"#,
        );
        let d = extract(&html, &url, "CLP").expect("product");
        assert_eq!(d.name, "Teclado");
        assert_eq!(d.price, 29990.0);
        assert_eq!(d.currency, "CLP");
        assert_eq!(d.external_product_id.as_deref(), Some("K1"));
        assert!(d.availability);
    }

    #[test]
    fn decodes_entities_in_names() {
        assert_eq!(clean_name("(TCG) &#8211; Pack &#38; Box"), "(TCG) – Pack & Box");
        assert_eq!(clean_name("  A   B  "), "A B");
    }

    #[test]
    fn jsonld_name_with_entities_is_decoded() {
        let url = Url::parse("https://tienda.cl/p/1").unwrap();
        let html = Html::parse_document(
            r#"<html><head><script type="application/ld+json">
            {"@type":"Product","name":"(TCG) &#8211; Pack &#38; Box",
            "offers":{"@type":"Offer","price":"69990","priceCurrency":"CLP"}}
            </script></head><body></body></html>"#,
        );
        let d = extract(&html, &url, "CLP").expect("product");
        assert_eq!(d.name, "(TCG) – Pack & Box");
    }

    #[test]
    fn domain_beats_stray_currency_code() {
        // A Chilean shop whose page mentions other currencies must stay CLP.
        let url = Url::parse("https://www.progaming.cl/p/1").unwrap();
        let text = "Envíos a todo Chile. Monedas: USD EUR ARS MXN CLP. Precio $21.990";
        assert_eq!(detect_currency(text, &url, "CLP"), "CLP");
    }

    #[test]
    fn adjacent_code_beats_domain() {
        // A .cl store legitimately charging in USD is detected via the price.
        let url = Url::parse("https://tienda.cl/p/1").unwrap();
        assert_eq!(detect_currency("Precio USD 19.99", &url, "CLP"), "USD");
    }

    #[test]
    fn loose_code_matches_whole_word_on_unknown_host() {
        let url = Url::parse("https://example.com/p/1").unwrap();
        assert_eq!(detect_currency("Moneda: ARS (peso argentino)", &url, "CLP"), "ARS");
    }

    #[test]
    fn code_inside_word_is_not_matched() {
        let url = Url::parse("https://example.com/p/1").unwrap();
        // "GUITARS" contains the bytes "ARS" but is not a currency marker.
        assert_eq!(detect_currency("Guitars and basses", &url, "CLP"), "CLP");
    }

    #[test]
    fn dom_price_prefers_sale_over_list_price() {
        let url = Url::parse("https://tienda.cl/p/1").unwrap();
        let html = Html::parse_document(
            r#"<html><body><h1>Silla gamer</h1>
            <span class="price">$199.990</span>
            <span class="price">$149.990</span></body></html>"#,
        );
        let d = extract(&html, &url, "CLP").expect("product");
        assert_eq!(d.price, 149990.0);
        assert_eq!(d.currency, "CLP");
    }
}
