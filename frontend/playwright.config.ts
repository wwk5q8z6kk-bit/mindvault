import { defineConfig } from '@playwright/test';

const HOST = '127.0.0.1';
const PORT = 4173;

export default defineConfig({
	webServer: {
		// `--host 127.0.0.1` is load-bearing. Without it `vite preview` binds
		// IPv6-only, so `curl localhost` (which resolves to ::1) succeeds while
		// Chromium — which tries 127.0.0.1 first — gets a refused connection and
		// then hangs until the test times out. Every e2e test failed on
		// `page.goto` with no console errors, which reads like an application
		// fault but is purely a bind-address problem.
		//
		// `pnpm exec vite preview`, not `pnpm run preview -- ...`: pnpm passes the
		// `--` through literally, so vite receives it as an argument and silently
		// ignores the flags after it — the server comes up on the default
		// IPv6-only binding and the flags appear to have no effect.
		//
		// pnpm, not npm: this project uses pnpm, and mixing the two re-resolves
		// the dependency tree on every run.
		command: `pnpm run build && pnpm exec vite preview --host ${HOST} --port ${PORT}`,
		url: `http://${HOST}:${PORT}/`,
		// A cold `vite build` plus preview startup exceeds the 60s default.
		timeout: 180_000,
		reuseExistingServer: !process.env.CI
	},
	// Specs use relative paths so the host is defined in exactly one place.
	use: { baseURL: `http://${HOST}:${PORT}` },
	testDir: 'e2e'
});
