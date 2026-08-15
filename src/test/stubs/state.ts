// src/test/stubs/state.ts — Stub de `$app/state` para tests.
// Se mapea en vitest.config.ts mediante alias para que las páginas puedan
// importar `page` sin el runtime de SvelteKit. `page.url` refleja
// `window.location` (jsdom), igual que en SvelteKit la URL de navegación.
export const page = {
	get url(): URL {
		return new URL(window.location.href);
	}
};
