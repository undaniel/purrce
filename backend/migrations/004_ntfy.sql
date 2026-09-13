-- Add ntfy support to notification_config
ALTER TABLE notification_config 
ADD COLUMN ntfy_enabled BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN ntfy_url VARCHAR(512) DEFAULT 'https://ntfy.sh',
ADD COLUMN ntfy_topic VARCHAR(255),
ADD COLUMN ntfy_token VARCHAR(255);
