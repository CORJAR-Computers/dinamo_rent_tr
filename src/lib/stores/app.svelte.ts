// app.svelte.ts — Store de información de la aplicación (versión real)
//
// Carga la versión real de la app desde el backend (comando `app_version`,
// fuente `package_info`: Cargo.toml / tauri.conf.json en el build). Reemplaza
// el literal v3.2.0 heredado del proyecto anterior en el menú lateral y el
// login. Best-effort: ante error se deja la versión vacía (la UI no muestra el
// sufijo "v…") sin romper nada.

import { appApi } from '$lib/api';

class AppStore {
	/** Versión real (p. ej. "1.0.16"); '' si aún no cargó o falló. */
	version = $state('');

	async cargarVersion(): Promise<void> {
		try {
			this.version = await appApi.version();
		} catch (e) {
			console.warn('No se pudo cargar la versión de la app:', e);
		}
	}
}

export const app = new AppStore();
