pub struct AppConfig {
    pub database_url: String,
    pub bind_addr: String,
    pub port: u16,
    pub default_currency: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://purrce:purrce@localhost:5432/purrce".to_string()),
            // Default to all interfaces so the Docker port mapping keeps working;
            // set BIND_ADDR=127.0.0.1 to restrict a bare-metal install to localhost.
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            default_currency: std::env::var("DEFAULT_CURRENCY")
                .unwrap_or_else(|_| "CLP".to_string())
                .to_uppercase(),
        })
    }
}
