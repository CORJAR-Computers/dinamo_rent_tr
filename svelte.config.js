import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		// SPA mode: Tauri sirve la app como archivos estáticos
		adapter: adapter({ fallback: 'index.html' }),
		prerender: { entries: [] }
	}
};

export default config;
