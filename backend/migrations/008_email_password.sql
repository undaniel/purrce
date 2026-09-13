-- SMTP password for email notifications
ALTER TABLE notification_config
ADD COLUMN IF NOT EXISTS email_smtp_password VARCHAR(255);
