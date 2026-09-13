-- One offer per (store, canonical URL). Prevents the same product URL from
-- being imported twice and creating duplicate products/watches/alerts.
--
-- First collapse any existing duplicates, keeping the oldest offer per pair
-- (its dependents win; newer duplicates and their history are removed).
DELETE FROM product_offers po
USING product_offers keep
WHERE po.store_id = keep.store_id
  AND po.url = keep.url
  AND (po.created_at, po.id) > (keep.created_at, keep.id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_product_offers_store_url
    ON product_offers (store_id, url);
