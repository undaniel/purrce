-- Notification configuration
CREATE TABLE notification_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    telegram_enabled BOOLEAN NOT NULL DEFAULT false,
    telegram_bot_token VARCHAR(255),
    telegram_chat_id VARCHAR(100),
    email_enabled BOOLEAN NOT NULL DEFAULT false,
    email_smtp_host VARCHAR(255),
    email_smtp_port INTEGER DEFAULT 587,
    email_from VARCHAR(255),
    email_to VARCHAR(255),
    webhook_enabled BOOLEAN NOT NULL DEFAULT false,
    webhook_url VARCHAR(1024),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default config
INSERT INTO notification_config (id) VALUES (uuid_generate_v4());
