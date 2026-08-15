import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

// Configuración de tests de frontend (Vitest + Testing Library).
// Se usa el plugin svelte (sin sveltekit) porque los tests montan componentes
// de forma aislada; el alias $lib y el mock de Tauri los resuelve el setup.
export default defineConfig({
	plugins: [svelte()],
	resolve: {
		alias: {
			$lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
			// El runtime de SvelteKit no existe en jsdom; las páginas que usen
			// `goto` (login) resuelven contra este stub.
			'$app/navigation': fileURLToPath(
				new URL('./src/test/stubs/navigation.ts', import.meta.url)
			),
			// Los load que usan `redirect` de @sveltejs/kit (cambiar-password)
			// resuelven contra este stub.
			'@sveltejs/kit': fileURLToPath(
				new URL('./src/test/stubs/sveltekit.ts', import.meta.url)
			),
			// `page` de $app/state (rutas que leen query params, p. ej. rentas
			// con ?desdeReserva=). Lee window.location de jsdom.
			'$app/state': fileURLToPath(
				new URL('./src/test/stubs/state.ts', import.meta.url)
			)
		},
		// Sin esto, Vitest resuelve 'svelte' a index-server.js y `mount()`
		// falla con lifecycle_function_unavailable. La condición browser
		// apunta a la implementación client (jsdom).
		conditions: ['browser']
	},
	test: {
		environment: 'jsdom',
		// Origen real → localStorage/sessionStorage disponibles (about:blank es opaco)
		environmentOptions: {
			jsdom: { url: 'http://localhost:5173/' }
		},
		globals: true,
		setupFiles: ['./src/test/setup.ts'],
		include: ['src/**/*.test.ts'],
		css: false
	}
});
