// app.ts — Comandos de aplicación (ventana / versión)
import { invokeCmd } from './base';

export const appApi = {
	/**
	 * Avisa al backend que el frontend ya escucha el evento de cierre
	 * (app-close-requested), para que la X de la ventana muestre el diálogo
	 * de confirmación en lugar de cerrar directamente.
	 */
	frontendLista: () => invokeCmd<void>('app_frontend_lista'),
	/** Versión real de la app (backend: package_info → Cargo.toml / tauri.conf.json). */
	version: () => invokeCmd<string>('app_version')
};
