use std::sync::OnceLock;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{info, warn};
use crate::api::handlers::notifications::NotificationConfig;

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();
fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
}

pub struct AlertPayload {
    pub product_name: String,
    pub store_name: String,
    pub old_price: f64,
    pub new_price: f64,
    pub currency: String,
    pub alert_type: String,
    pub offer_url: String,
}

impl AlertPayload {
    fn title(&self) -> String {
        match self.alert_type.as_str() {
            "PRICE_DECREASE" => format!("↓ Bajó el precio: {}", self.product_name),
            "HISTORICAL_LOW" => format!("🔥 Mínimo histórico: {}", self.product_name),
            "BELOW_PRICE" => format!("✓ Bajo el umbral: {}", self.product_name),
            "BACK_IN_STOCK" => format!("● Volvió al stock: {}", self.product_name),
            "OUT_OF_STOCK" => format!("○ Sin stock: {}", self.product_name),
            _ => format!("Alerta: {}", self.product_name),
        }
    }

    fn body(&self) -> String {
        let change_pct = if self.old_price > 0.0 {
            let pct = (self.new_price - self.old_price) / self.old_price * 100.0;
            format!(" ({:+.1}%)", pct)
        } else {
            String::new()
        };
        format!(
            "{} — {} {:.0} → {:.0}{}",
            self.store_name, self.currency, self.old_price, self.new_price, change_pct
        )
    }
}

pub async fn dispatch(config: &NotificationConfig, payload: &AlertPayload) {
    if config.webhook_enabled {
        if let Some(url) = &config.webhook_url {
            send_webhook(url, payload).await;
        }
    }
    if config.telegram_enabled {
        if let (Some(token), Some(chat_id)) = (&config.telegram_bot_token, &config.telegram_chat_id) {
            send_telegram(token, chat_id, payload).await;
        }
    }
    if config.ntfy_enabled {
        if let Some(topic) = &config.ntfy_topic {
            let base = config.ntfy_url.as_deref().unwrap_or("https://ntfy.sh");
            send_ntfy(base, topic, config.ntfy_token.as_deref(), payload).await;
        }
    }
    if config.email_enabled {
        if let (Some(host), Some(from), Some(to), Some(password)) = (
            &config.email_smtp_host,
            &config.email_from,
            &config.email_to,
            &config.email_smtp_password,
        ) {
            let port = config.email_smtp_port.unwrap_or(587);
            send_email(host, port, from, to, password, payload).await;
        }
    }
}

async fn send_webhook(url: &str, payload: &AlertPayload) {
    let client = http();
    let body = serde_json::json!({
        "title": payload.title(),
        "body": payload.body(),
        "product_name": payload.product_name,
        "store_name": payload.store_name,
        "old_price": payload.old_price,
        "new_price": payload.new_price,
        "currency": payload.currency,
        "alert_type": payload.alert_type,
        "offer_url": payload.offer_url,
    });
    match client.post(url).json(&body).send().await {
        Ok(r) if r.status().is_success() => info!("Webhook sent to {}", url),
        Ok(r) => warn!("Webhook {} returned {}", url, r.status()),
        Err(e) => warn!("Webhook {} failed: {}", url, e),
    }
}

async fn send_telegram(token: &str, chat_id: &str, payload: &AlertPayload) {
    let client = http();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let text = format!(
        "<b>{}</b>\n{}\n\n<a href=\"{}\">Ver producto</a>",
        payload.title(), payload.body(), payload.offer_url
    );
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
    });
    match client.post(&url).json(&body).send().await {
        Ok(r) if r.status().is_success() => info!("Telegram notification sent"),
        Ok(r) => warn!("Telegram returned {}", r.status()),
        Err(e) => warn!("Telegram failed: {}", e),
    }
}

async fn send_email(host: &str, port: u16, from: &str, to: &str, password: &str, payload: &AlertPayload) {
    let body = format!("{}\n\n{}\n\nVer producto: {}", payload.title(), payload.body(), payload.offer_url);
    let msg = match Message::builder()
        .from(from.parse().unwrap_or_else(|_| "purrce@localhost".parse().unwrap()))
        .to(to.parse().unwrap_or_else(|_| "user@localhost".parse().unwrap()))
        .subject(payload.title())
        .header(ContentType::TEXT_PLAIN)
        .body(body)
    {
        Ok(m) => m,
        Err(e) => { warn!("Failed to build email: {}", e); return; }
    };

    let creds = Credentials::new(from.to_string(), password.to_string());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
        .map(|b| b.port(port).credentials(creds).build());

    match mailer {
        Ok(m) => match m.send(msg).await {
            Ok(_) => info!("Email notification sent to {}", to),
            Err(e) => warn!("Email send failed: {}", e),
        },
        Err(e) => warn!("Email transport error: {}", e),
    }
}

async fn send_ntfy(base_url: &str, topic: &str, token: Option<&str>, payload: &AlertPayload) {
    let client = http();
    let url = format!("{}/{}", base_url.trim_end_matches('/'), topic);
    let mut req = client
        .post(&url)
        .header("Title", payload.title())
        .header("Tags", "bell,price")
        .header("Click", &payload.offer_url)
        .body(payload.body());
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    match req.send().await {
        Ok(r) if r.status().is_success() => info!("ntfy notification sent"),
        Ok(r) => warn!("ntfy returned {}", r.status()),
        Err(e) => warn!("ntfy failed: {}", e),
    }
}
