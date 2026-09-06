const HTML_HEADERS = {
	'cache-control': 'no-cache',
	'content-type': 'text/html; charset=utf-8'
};

function withPath(request, pathname) {
	const url = new URL(request.url);
	url.pathname = pathname;
	return new Request(url, request);
}

async function fetchAsset(request, assets) {
	const response = await assets.fetch(request);
	if (response.status !== 404) return response;

	const pathname = new URL(request.url).pathname;
	if (!pathname.includes('.')) {
		const routeResponse = await assets.fetch(withPath(request, `${pathname.replace(/\/$/, '')}.html`));
		if (routeResponse.status !== 404) return routeResponse;
	}

	const fallback = await assets.fetch(withPath(request, '/index.html'));
	return new Response(fallback.body, {
		headers: HTML_HEADERS,
		status: fallback.status
	});
}

export default {
	async fetch(request, env) {
		const url = new URL(request.url);

		if (url.pathname === '/') {
			url.pathname = '/notes';
			return Response.redirect(url.toString(), 302);
		}

		if (!env.ASSETS?.fetch) {
			return new Response('Static asset service unavailable.', { status: 503 });
		}

		return fetchAsset(request, env.ASSETS);
	}
};
