export interface Tag {
  id: string;
  name: string;
  color: string;
}

export interface TagWithUsage extends Tag {
  product_count: number;
}

export interface Product {
  id: string;
  name: string;
  description: string | null;
  image_url: string | null;
  offers: Offer[];
  tags: Tag[];
  created_at: string;
  updated_at: string;
}

export interface Offer {
  id: string;
  product_id: string;
  store_id: string;
  store_name: string;
  url: string;
  currency: string;
  current_price: number;
  availability: boolean;
  last_checked_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface PriceHistory {
  id: string;
  offer_id: string;
  price: number;
  currency: string;
  availability: boolean;
  observed_at: string;
}

export interface Watch {
  id: string;
  offer_id: string;
  enabled: boolean;
  interval_seconds: number;
  last_check_at: string | null;
  next_check_at: string | null;
  status: string;
  failure_count: number;
  created_at: string;
  updated_at: string;
  product_id?: string;
  product_name?: string;
  store_name?: string;
  offer_url?: string;
  current_price?: number;
  currency?: string;
  availability?: boolean;
}

export interface Alert {
  id: string;
  offer_id: string;
  alert_type: string;
  threshold_price: number | null;
  enabled: boolean;
  triggered_at: string | null;
  created_at: string;
  product_name?: string;
  product_image?: string;
  store_name?: string;
  offer_url?: string;
}

export interface Comparison {
  product: Product;
  offers: Offer[];
  best_offer: Offer | null;
  min_price: number;
  max_price: number;
  savings: number;
}

export interface ApiError {
  error: {
    code: string;
    message: string;
  };
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  limit: number;
  total_pages: number;
}

export interface CurrencyValue {
  currency: string;
  total: number;
  count: number;
}

export interface ProductStats {
  total: number;
  down: number;
  up: number;
  unchanged: number;
  at_min: number;
  no_price: number;
  values: CurrencyValue[];
}

export interface ProductList {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface ListItem {
  id: string;
  list_id: string;
  product_id: string;
  quantity: number;
  created_at: string;
  product_name: string;
  image_url: string | null;
  best_price: number | null;
  currency: string | null;
}

export interface ListWithTotal {
  list: ProductList;
  items: ListItem[];
  total_cost: number;
}
