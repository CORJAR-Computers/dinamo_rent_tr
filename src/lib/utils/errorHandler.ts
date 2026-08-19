// errorHandler.ts — Captura errores JS globales y los envía al backend
//
// Se inicializa una sola vez (en el layout o en el bootstrap de la app).
// Usa un debounce para no saturar el backend si hay un bucle de errores.
//
// Patrón: window.onerror + window.onunhandledrejection → logApi.registrar_error

import { logApi } from '$lib/api';

let initialized = false;
let lastErrorTime = 0;
const DEBOUNCE_MS = 1000; // Máximo 1 error por segundo

/**
 * Inicializa el handler global de errores JS.
 * Llamar una sola vez al arrancar la app (layout o bootstrap).
 * Requiere que haya una sesión activa (sessionId).
 */
export function initErrorHandler(getSessionId: () => string | null) {
	if (initialized || typeof window === 'undefined') return;
	initialized = true;

	// Errores JS no capturados (sincronos)
	window.onerror = (mensaje, fuente, linea, columna, error) => {
		const now = Date.now();
		if (now - lastErrorTime < DEBOUNCE_MS) return;
		lastErrorTime = now;

		const sessionId = getSessionId();
		if (!sessionId) return;

		const msg = typeof mensaje === 'string' ? mensaje : String(mensaje);
		const stack = error instanceof Error ? error.stack : undefined;

		// Fire and forget — no bloqueamos la UI
		logApi
			.registrarError(sessionId, `[window.onerror] ${msg}`, stack, fuente || undefined, linea, columna)
			.catch(() => {});
	};

	// Promesas rechazadas sin catch
	window.onunhandledrejection = (event: PromiseRejectionEvent) => {
		const now = Date.now();
		if (now - lastErrorTime < DEBOUNCE_MS) return;
		lastErrorTime = now;

		const sessionId = getSessionId();
		if (!sessionId) return;

		const reason = event.reason;
		let msg: string;
		let stack: string | undefined;

		if (reason instanceof Error) {
			msg = reason.message;
			stack = reason.stack;
		} else if (typeof reason === 'string') {
			msg = reason;
			stack = undefined;
		} else {
			msg = JSON.stringify(reason);
			stack = undefined;
		}

		logApi
			.registrarError(sessionId, `[unhandledrejection] ${msg}`, stack)
			.catch(() => {});
	};
}
