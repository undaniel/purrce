import type { Product, Offer, PriceHistory, Watch, Alert, Tag, TagWithUsage, Comparison, ApiError, PaginatedResponse, ProductStats, ProductList, ListWithTotal } from '$lib/types';

const BASE_URL = '/api';

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${BASE_URL}${url}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers
    }
  });

  if (!response.ok) {
    const body = await response.json();
    // Backend sends {code, message} directly (not nested under "error")
    const err = new Error(body.message || body.error?.message || 'Error desconocido') as any;
    err.code = body.code || body.error?.code;
    err.status = response.status;
    throw err;
  }

  return response.json();
}

function buildPaginationParams(page?: number, limit?: number): string {
  const params = new URLSearchParams();
  if (page !== undefined) params.set('page', page.toString());
  if (limit !== undefined) params.set('limit', limit.toString());
  const str = params.toString();
  return str ? `?${str}` : '';
}

function buildProductParams(page?: number, limit?: number, search?: string, tag?: string): string {
  const params = new URLSearchParams();
  if (page !== undefined) params.set('page', page.toString());
  if (limit !== undefined) params.set('limit', limit.toString());
  if (search) params.set('search', search);
  if (tag) params.set('tag', tag);
  const str = params.toString();
  return str ? `?${str}` : '';
}

export const api = {
  products: {
    list: (page?: number, limit?: number, search?: string, tag?: string) => 
      fetchJson<PaginatedResponse<Product>>(`/products${buildProductParams(page, limit, search, tag)}`),
    listAll: () => fetchJson<Product[]>('/products/all'),
    stats: () => fetchJson<ProductStats>('/products/stats'),
    get: (id: string) => fetchJson<Product>(`/products/${id}`),
    import: (url: string, watch?: boolean, interval?: number, tags?: string[], name?: string, price?: number, imageUrl?: string) =>
      fetchJson<Product>('/products/import', {
        method: 'POST',
        body: JSON.stringify({ url, watch, interval_seconds: interval, tags, name, price, image_url: imageUrl })
      }),
    update: (id: string, data: { name?: string; description?: string; image_url?: string }) =>
      fetchJson<Product>(`/products/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data)
      }),
    delete: (id: string) =>
      fetchJson<{ deleted: boolean }>(`/products/${id}`, { method: 'DELETE' }),
    addOffer: (id: string, url: string, watch?: boolean, interval?: number) =>
      fetchJson<Offer>(`/products/${id}/offers`, {
        method: 'POST',
        body: JSON.stringify({ url, watch, interval_seconds: interval })
      }),
    comparison: (id: string) => fetchJson<Comparison>(`/products/${id}/comparison`),
    getTags: (id: string) => fetchJson<Tag[]>(`/products/${id}/tags`),
    setTags: (id: string, tags: string[]) =>
      fetchJson<Tag[]>(`/products/${id}/tags`, {
        method: 'POST',
        body: JSON.stringify({ tags })
      }),
  },

  offers: {
    get: (id: string) => fetchJson<Offer>(`/offers/${id}`),
    history: (id: string, page?: number, limit?: number) =>
      fetchJson<PaginatedResponse<PriceHistory>>(`/offers/${id}/history${buildPaginationParams(page, limit)}`),
    historyBulk: (ids: string[], limit = 120) => {
      const params = new URLSearchParams();
      params.set('ids', ids.join(','));
      params.set('limit', limit.toString());
      return fetchJson<PriceHistory[]>(`/price-history?${params.toString()}`);
    },
    updatePrice: (id: string, data: { price: number; currency?: string; availability?: boolean }) =>
      fetchJson<Offer>(`/offers/${id}/price`, {
        method: 'PATCH',
        body: JSON.stringify(data)
      })
  },

  watches: {
    list: (page?: number, limit?: number) => fetchJson<PaginatedResponse<Watch>>(`/watches${buildPaginationParams(page, limit)}`),
    create: (offerId: string, interval?: number) =>
      fetchJson<Watch>('/watches', {
        method: 'POST',
        body: JSON.stringify({ offer_id: offerId, interval_seconds: interval })
      }),
    update: (id: string, data: { enabled?: boolean; interval_seconds?: number }) =>
      fetchJson<Watch>(`/watches/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(data)
      }),
    delete: (id: string) =>
      fetchJson<{ deleted: boolean }>(`/watches/${id}`, { method: 'DELETE' }),
    check: (id: string) =>
      fetchJson<{ status: string; watch_id: string }>(`/watches/${id}/check`, { method: 'POST' }),
    resume: (id: string) =>
      fetchJson<Watch>(`/watches/${id}/resume`, { method: 'POST' }),
  },

  alerts: {
    list: (page?: number, limit?: number) => fetchJson<PaginatedResponse<Alert>>(`/alerts${buildPaginationParams(page, limit)}`),
    create: (offerId: string, alertType: string, thresholdPrice?: number | null) =>
      fetchJson<Alert>('/alerts', {
        method: 'POST',
        body: JSON.stringify({ offer_id: offerId, alert_type: alertType, threshold_price: thresholdPrice })
      }),
    update: (id: string, data: { enabled?: boolean; threshold_price?: number }) =>
      fetchJson<Alert>(`/alerts/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(data)
      })
  },

  tags: {
    list: () => fetchJson<Tag[]>('/tags'),
    listWithUsage: () => fetchJson<TagWithUsage[]>('/tags/usage'),
    create: (name: string, color?: string) =>
      fetchJson<Tag>('/tags', {
        method: 'POST',
        body: JSON.stringify({ name, color })
      }),
    update: (id: string, data: { name?: string; color?: string }) =>
      fetchJson<Tag>(`/tags/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(data)
      }),
    merge: (id: string, targetId: string) =>
      fetchJson<{ merged: boolean }>(`/tags/${id}/merge`, {
        method: 'POST',
        body: JSON.stringify({ target_id: targetId })
      }),
    delete: (id: string) =>
      fetchJson<{ deleted: boolean }>(`/tags/${id}`, { method: 'DELETE' }),
  },

  lists: {
    list: () => fetchJson<ProductList[]>('/lists'),
    get: (id: string) => fetchJson<ListWithTotal>(`/lists/${id}`),
    create: (name: string) => fetchJson<ProductList>('/lists', {
      method: 'POST',
      body: JSON.stringify({ name })
    }),
    rename: (id: string, name: string) => fetchJson<ProductList>(`/lists/${id}`, {
      method: 'PATCH',
      body: JSON.stringify({ name })
    }),
    delete: (id: string) => fetchJson<{ deleted: boolean }>(`/lists/${id}`, { method: 'DELETE' }),
    addItem: (listId: string, productId: string, quantity = 1) =>
      fetchJson<{ ok: boolean }>(`/lists/${listId}/items`, {
        method: 'POST',
        body: JSON.stringify({ product_id: productId, quantity })
      }),
    updateQuantity: (listId: string, productId: string, quantity: number) =>
      fetchJson<{ ok: boolean }>(`/lists/${listId}/items/${productId}`, {
        method: 'PATCH',
        body: JSON.stringify({ quantity })
      }),
    removeItem: (listId: string, productId: string) =>
      fetchJson<{ deleted: boolean }>(`/lists/${listId}/items/${productId}`, { method: 'DELETE' }),
  }
};
