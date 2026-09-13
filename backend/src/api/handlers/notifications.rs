use std::sync::OnceLock;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::api::dto::ErrorResponse;
use crate::api::AppState;

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();
fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationConfig {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub email_enabled: bool,
    pub email_smtp_host: Option<String>,
    pub email_smtp_port: Option<u16>,
    pub email_smtp_password: Option<String>,
    pub email_from: Option<String>,
    pub email_to: Option<String>,
    pub webhook_enabled: bool,
    pub webhook_url: Option<String>,
    pub ntfy_enabled: bool,
    pub ntfy_url: Option<String>,
    pub ntfy_topic: Option<String>,
    pub ntfy_token: Option<String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
            email_enabled: false,
            email_smtp_host: None,
            email_smtp_port: Some(587),
            email_smtp_password: None,
            email_from: None,
            email_to: None,
            webhook_enabled: false,
            webhook_url: None,
            ntfy_enabled: false,
            ntfy_url: Some("https://ntfy.sh".to_string()),
            ntfy_topic: None,
            ntfy_token: None,
        }
    }
}

impl From<crate::infrastructure::database::repository::NotificationConfigRow> for NotificationConfig {
    fn from(row: crate::infrastructure::database::repository::NotificationConfigRow) -> Self {
        Self {
            telegram_enabled: row.telegram_enabled,
            telegram_bot_token: row.telegram_bot_token,
            telegram_chat_id: row.telegram_chat_id,
            email_enabled: row.email_enabled,
            email_smtp_host: row.email_smtp_host,
            email_smtp_port: row.email_smtp_port.map(|p| p as u16),
            email_smtp_password: row.email_smtp_password,
            email_from: row.email_from,
            email_to: row.email_to,
            webhook_enabled: row.webhook_enabled,
            webhook_url: row.webhook_url,
            ntfy_enabled: row.ntfy_enabled,
            ntfy_url: row.ntfy_url,
            ntfy_topic: row.ntfy_topic,
            ntfy_token: row.ntfy_token,
        }
    }
}

/// Placeholder returned in place of stored secrets. The frontend echoes it back
/// untouched; `resolve_secret` then keeps the real value.
const SECRET_MASK: &str = "********";

fn mask_secret(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty()).map(|_| SECRET_MASK.to_string())
}

/// Decide the value to persist: `None`/mask keeps the existing secret, empty
/// clears it, anything else replaces it.
fn resolve_secret(incoming: Option<&str>, existing: Option<String>) -> Option<String> {
    match incoming {
        None => existing,
        Some(v) if v == SECRET_MASK => existing,
        Some("") => None,
        Some(v) => Some(v.to_string()),
    }
}

fn masked(mut config: NotificationConfig) -> NotificationConfig {
    config.telegram_bot_token = mask_secret(config.telegram_bot_token);
    config.email_smtp_password = mask_secret(config.email_smtp_password);
    config.ntfy_token = mask_secret(config.ntfy_token);
    config
}

pub async fn get_config(State(state): State<AppState>) -> Result<Json<NotificationConfig>, ErrorResponse> {
    match state.repo.get_notification_config().await {
        Ok(Some(row)) => Ok(Json(masked(NotificationConfig::from(row)))),
        _ => {
            let config = state.notification_config.read().await;
            Ok(Json(masked(config.clone())))
        }
    }
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<NotificationConfig>,
) -> Result<Json<NotificationConfig>, ErrorResponse> {
    // Keep the stored secret when the client sends back the mask.
    let existing = state.repo.get_notification_config().await.ok().flatten();
    let (old_tg, old_smtp, old_ntfy) = existing
        .map(|r| (r.telegram_bot_token, r.email_smtp_password, r.ntfy_token))
        .unwrap_or((None, None, None));

    let resolved = NotificationConfig {
        telegram_bot_token: resolve_secret(req.telegram_bot_token.as_deref(), old_tg),
        email_smtp_password: resolve_secret(req.email_smtp_password.as_deref(), old_smtp),
        ntfy_token: resolve_secret(req.ntfy_token.as_deref(), old_ntfy),
        ..req.clone()
    };

    state.repo.update_notification_config(
        resolved.telegram_enabled,
        resolved.telegram_bot_token.as_deref(),
        resolved.telegram_chat_id.as_deref(),
        resolved.email_enabled,
        resolved.email_smtp_host.as_deref(),
        resolved.email_smtp_port.map(|p| p as i32),
        resolved.email_from.as_deref(),
        resolved.email_to.as_deref(),
        resolved.email_smtp_password.as_deref(),
        resolved.webhook_enabled,
        resolved.webhook_url.as_deref(),
        resolved.ntfy_enabled,
        resolved.ntfy_url.as_deref(),
        resolved.ntfy_topic.as_deref(),
        resolved.ntfy_token.as_deref(),
    )
    .await
    .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let mut config = state.notification_config.write().await;
    *config = resolved.clone();

    Ok(Json(masked(resolved)))
}

pub async fn test_telegram(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let config = get_config_from_db(&state).await;
    
    if !config.telegram_enabled {
        return Err(ErrorResponse::bad_request("Telegram no está habilitado"));
    }
    
    let token = config.telegram_bot_token.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("Token de bot no configurado"))?;
    let chat_id = config.telegram_chat_id.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("Chat ID no configurado"))?;
    
    let client = http();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    
    let response = client.post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": "🔔 Purrce - Notificación de prueba\n\nSi recibes este mensaje, las notificaciones están configuradas correctamente.",
            "parse_mode": "HTML"
        }))
        .send()
        .await
        .map_err(|e| ErrorResponse::bad_gateway(format!("Error al enviar mensaje: {}", e)))?;
    
    if response.status().is_success() {
        Ok(Json(serde_json::json!({ "success": true, "message": "Mensaje enviado correctamente" })))
    } else {
        let error = response.text().await.unwrap_or_default();
        Err(ErrorResponse::bad_gateway(format!("Error de Telegram: {}", error)))
    }
}

pub async fn test_ntfy(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let config = get_config_from_db(&state).await;
    
    if !config.ntfy_enabled {
        return Err(ErrorResponse::bad_request("ntfy no está habilitado"));
    }
    
    let topic = config.ntfy_topic.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("Tema no configurado"))?;
    let url = config.ntfy_url.as_deref().unwrap_or("https://ntfy.sh");
    
    let client = http();
    let ntfy_url = format!("{}/{}", url, topic);
    
    let mut request = client.post(&ntfy_url)
        .header("Title", "Purrce - Notificación de prueba")
        .header("Tags", "bell,price")
        .body("Si recibes este mensaje, las notificaciones están configuradas correctamente.");
    
    if let Some(token) = &config.ntfy_token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    let response = request.send()
        .await
        .map_err(|e| ErrorResponse::bad_gateway(format!("Error al enviar mensaje: {}", e)))?;
    
    if response.status().is_success() {
        Ok(Json(serde_json::json!({ "success": true, "message": "Mensaje enviado correctamente" })))
    } else {
        let error = response.text().await.unwrap_or_default();
        Err(ErrorResponse::bad_gateway(format!("Error de ntfy: {}", error)))
    }
}

pub async fn test_email(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let config = get_config_from_db(&state).await;

    if !config.email_enabled {
        return Err(ErrorResponse::bad_request("Email no está habilitado"));
    }
    let host = config.email_smtp_host.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("SMTP host no configurado"))?;
    let from = config.email_from.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("Email from no configurado"))?;
    let to = config.email_to.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("Email to no configurado"))?;
    let password = config.email_smtp_password.as_ref()
        .ok_or_else(|| ErrorResponse::bad_request("SMTP password no configurado"))?;

    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let msg = Message::builder()
        .from(from.parse().map_err(|_| ErrorResponse::bad_request("Email from inválido"))?)
        .to(to.parse().map_err(|_| ErrorResponse::bad_request("Email to inválido"))?)
        .subject("Purrce - Notificación de prueba")
        .header(ContentType::TEXT_PLAIN)
        .body("Si recibes este mensaje, las notificaciones de email están configuradas correctamente.".to_string())
        .map_err(|e| ErrorResponse::bad_request(&format!("Email inválido: {}", e)))?;

    let port = config.email_smtp_port.unwrap_or(587);
    let creds = Credentials::new(from.clone(), password.clone());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
        .map_err(|e| ErrorResponse::bad_gateway(format!("SMTP host inválido: {}", e)))?
        .port(port)
        .credentials(creds)
        .build();

    mailer.send(msg).await
        .map(|_| Json(serde_json::json!({ "success": true, "message": "Email enviado correctamente" })))
        .map_err(|e| ErrorResponse::bad_gateway(format!("Error al enviar email: {}", e)))
}

async fn get_config_from_db(state: &AppState) -> NotificationConfig {
    match state.repo.get_notification_config().await {
        Ok(Some(row)) => NotificationConfig::from(row),
        _ => {
            let config = state.notification_config.read().await;
            config.clone()
        }
    }
}
