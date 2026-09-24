export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Als het een API verzoek is -> stuur door naar Pterodactyl server via HTTP
    if (url.pathname.startsWith('/api')) {
      url.protocol = 'http:';
      url.hostname = 'node4.eu.codehost.me';
      url.port = '2002';

      const headers = new Headers(request.headers);
      headers.set('Host', 'node4.eu.codehost.me:2002');

      const init = {
        method: request.method,
        headers: headers,
        redirect: 'follow',
      };

      if (request.method !== 'GET' && request.method !== 'HEAD') {
        init.body = request.body;
      }

      try {
        const response = await fetch(url.toString(), init);
        const newHeaders = new Headers(response.headers);
        newHeaders.set('Access-Control-Allow-Origin', '*');
        newHeaders.set('Access-Control-Allow-Methods', 'GET, POST, DELETE, OPTIONS');
        newHeaders.set('Access-Control-Allow-Headers', '*');

        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers: newHeaders,
        });
      } catch (err) {
        return new Response(JSON.stringify({ error: 'Backend niet bereikbaar: ' + (err.message || err) }), {
          status: 502,
          headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
        });
      }
    }

    // Serveer statische bestanden
    if (env && env.ASSETS) {
      return env.ASSETS.fetch(request);
    }

    return new Response('Niet gevonden', { status: 404 });
  }
};
