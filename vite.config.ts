import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

// https://vite.dev/config/
export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true,
		// Tauri espera un puerto fijo y no permite cambios aleatorios
		watch: {
			ignored: ['**/src-tauri/**']
		}
	}
});
