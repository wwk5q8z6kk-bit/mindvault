import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			fallback: 'index.html'
		}),
		prerender: {
			entries: ['*'],
			handleMissingId: 'ignore',
			handleHttpError: 'ignore',
			handleUnseenRoutes: 'ignore'
		}
	}
};

export default config;
