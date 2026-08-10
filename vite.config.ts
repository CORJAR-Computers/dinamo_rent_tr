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
			ignored: [
				'**/src-tauri/**',
				// data/ contiene la BD Firebird dev, config.ini y los reportes del
				// Agente SIMIT (data/informes_simit/*.html). Vite recarga la página
				// completa al detectar cambios en esos archivos; excluirlos evita
				// el reload en cada corrida SIMIT y al tocar config/BD en dev.
				'**/data/**'
			]
		}
	}
});
