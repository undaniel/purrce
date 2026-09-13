-- Performance indexes for pagination and common queries

-- Products: ordering by created_at
CREATE INDEX IF NOT EXISTS idx_products_created_at ON products(created_at DESC);

-- Product offers: lookups by product_id and store_id
CREATE INDEX IF NOT EXISTS idx_product_offers_product_id ON product_offers(product_id);
CREATE INDEX IF NOT EXISTS idx_product_offers_store_id ON product_offers(store_id);

-- Price history: lookups by offer_id and time range
CREATE INDEX IF NOT EXISTS idx_price_history_offer_id ON price_history(offer_id);
CREATE INDEX IF NOT EXISTS idx_price_history_observed_at ON price_history(observed_at DESC);
CREATE INDEX IF NOT EXISTS idx_price_history_offer_time ON price_history(offer_id, observed_at DESC);

-- Watches: ordering and next check lookups
CREATE INDEX IF NOT EXISTS idx_watches_created_at ON watches(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_watches_next_check ON watches(next_check_at) WHERE enabled = true;
CREATE INDEX IF NOT EXISTS idx_watches_status ON watches(status);

-- Alerts: ordering and lookups by offer_id
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_offer_id ON alerts(offer_id);
CREATE INDEX IF NOT EXISTS idx_alerts_enabled ON alerts(enabled) WHERE enabled = true;

-- Tags: lookups by product
CREATE INDEX IF NOT EXISTS idx_product_tags_product_id ON product_tags(product_id);
CREATE INDEX IF NOT EXISTS idx_product_tags_tag_id ON product_tags(tag_id);

-- Stores: lookups by domain
CREATE INDEX IF NOT EXISTS idx_stores_domain ON stores(domain);
