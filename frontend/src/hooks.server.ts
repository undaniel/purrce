import type { Handle } from '@sveltejs/kit';

const API_URL = process.env.API_URL || 'http://localhost:8080';

export const handle: Handle = async ({ event, resolve }) => {
  if (event.url.pathname.startsWith('/api')) {
    const url = `${API_URL}${event.url.pathname}${event.url.search}`;
    
    const headers = new Headers();
    event.request.headers.forEach((value, key) => {
      if (key !== 'host') {
        headers.set(key, value);
      }
    });

    try {
      const response = await fetch(url, {
        method: event.request.method,
        headers,
        body: event.request.method !== 'GET' && event.request.method !== 'HEAD' 
          ? await event.request.text() 
          : undefined,
      });

      const responseHeaders = new Headers();
      response.headers.forEach((value, key) => {
        if (!['transfer-encoding', 'content-encoding'].includes(key.toLowerCase())) {
          responseHeaders.set(key, value);
        }
      });

      return new Response(await response.text(), {
        status: response.status,
        headers: responseHeaders,
      });
    } catch (err) {
      return new Response(JSON.stringify({ error: { code: 'PROXY_ERROR', message: 'Backend unavailable' } }), {
        status: 502,
        headers: { 'Content-Type': 'application/json' },
      });
    }
  }

  return resolve(event);
};
