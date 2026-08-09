// src/test/stubs/sveltekit.ts — Stub de `@sveltejs/kit` para tests.
// Se mapea en vitest.config.ts mediante alias. Exporta `redirect` con el
// mismo contrato del runtime real: lanza un error de navegación que los
// tests (o el propio SvelteKit) interpretan para abortar el render.
export function redirect(status: number, location: string): never {
	throw Object.assign(new Error('redirect'), { status, location });
}
