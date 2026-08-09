// src/test/tauri.ts — Puente tipado hacia el mock de `invoke` de Tauri.
// El estado real vive en src/test/setup.ts (vi.hoisted); aquí solo se delega
// para que los tests puedan registrar respuestas por comando sin importar setup.
export interface TauriInvoke {
	/**
	 * Registra el handler para un comando Tauri.
	 * El handler recibe los args y devuelve el valor de éxito,
	 * o lanza (string JSON o Error) para simular un error del backend.
	 * `any` relaja la contravarianza estricta para aceptar mocks con
	 * parámetros tipados ({ sessionId, datos }), manteniendo la tupla
	 * `calls` de vi.fn con su tipo exacto en los tests.
	 */
	register(cmd: string, fn: (args?: any) => unknown): void;
	/** Limpia todos los handlers registrados (se llama solo en afterEach) */
	reset(): void;
}

const bridge = (): TauriInvoke => {
	const state = (globalThis as unknown as { __tauriTestState?: TauriInvoke }).__tauriTestState;
	if (!state) {
		throw new Error(
			'[test] El mock de Tauri no está inicializado. Verifica que src/test/setup.ts esté en setupFiles de vitest.config.ts.'
		);
	}
	return state;
};

export const tauri: TauriInvoke = {
	register: (cmd, fn) => bridge().register(cmd, fn),
	reset: () => bridge().reset()
};
