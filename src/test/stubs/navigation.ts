// src/test/stubs/navigation.ts — Stub de `$app/navigation` para tests.
// Se mapea en vitest.config.ts mediante alias para que las páginas
// (login) puedan importar `goto` sin el runtime de SvelteKit.
import { vi } from 'vitest';

export const goto = vi.fn();
export const invalidate = vi.fn();
export const invalidateAll = vi.fn();
export const replaceState = vi.fn();
export const pushState = vi.fn();
export const beforeNavigate = vi.fn();
export const afterNavigate = vi.fn();

// Nota: `redirect` de @sveltejs/kit tiene su propio stub en sveltekit.ts
// (el load de cambiar-password lo importa desde @sveltejs/kit).
