import { writable } from 'svelte/store';

export type AppEvent =
  | { type: 'price_updated'; offer_id: string; new_price: number; currency: string }
  | { type: 'watch_status_changed'; watch_id: string; status: string }
  | { type: 'alert_triggered'; alert_id: string; offer_id: string; alert_type: string };

// Last received event — components react to this
export const lastEvent = writable<AppEvent | null>(null);

let es: EventSource | null = null;

export function startEventStream() {
  if (es) return;
  es = new EventSource('/api/events');
  es.onmessage = (e) => {
    try {
      const event: AppEvent = JSON.parse(e.data);
      lastEvent.set(event);
    } catch { /* ignore malformed */ }
  };
  es.onerror = () => {
    // Browser auto-reconnects SSE; just log quietly
    console.debug('SSE reconnecting...');
  };
}

export function stopEventStream() {
  es?.close();
  es = null;
}
