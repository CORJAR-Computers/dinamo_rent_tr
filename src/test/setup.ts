// src/test/setup.ts — Setup global de Vitest (jsdom).
// - Registra los matchers de jest-dom (toBeInTheDocument, etc.)
// - Mockea el core de Tauri: `invoke` delega en un registro de handlers
//   que los tests alimentan vía `tauri` (src/test/tauri.ts).
// - Limpia DOM / localStorage / sesión entre tests.
import '@testing-library/jest-dom/vitest';
import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/svelte';

// jsdom 30 no expone localStorage en este entorno (sessionStorage sí).
// El store de sesión usa `localStorage` como global; aportamos un shim en
// memoria idéntico al de un navegador.
function makeStorage(): Storage {
	const map = new Map<string, string>();
	return {
		get length() {
			return map.size;
		},
		clear: () => map.clear(),
		getItem: (k: string) => (map.has(k) ? (map.get(k) as string) : null),
		key: (i: number) => Array.from(map.keys())[i] ?? null,
		removeItem: (k: string) => void map.delete(k),
		setItem: (k: string, v: string) => void map.set(k, String(v))
	};
}
if (typeof window !== 'undefined' && typeof globalThis.localStorage === 'undefined') {
	const shim = makeStorage();
	globalThis.localStorage = shim;
	globalThis.sessionStorage = globalThis.sessionStorage ?? makeStorage();
	Object.defineProperty(window, 'localStorage', { value: shim, configurable: true });
}

// El estado del mock DEBE definirse con vi.hoisted dentro de este archivo:
// Vitest reordena el factory de vi.mock al inicio del módulo, así que las
// variables normales aún no existirían cuando el factory se registre.
const tauriState = vi.hoisted(() => {
	const handlers = new Map<string, (args?: any) => unknown>();

	return {
		register: (cmd: string, fn: (args?: any) => unknown): void => {
			handlers.set(cmd, fn);
		},
		reset: (): void => {
			handlers.clear();
		},
		invoke: (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
			const fn = handlers.get(cmd);
			if (!fn) {
				return Promise.reject(
					new Error(`[test] No hay mock registrado para el comando '${cmd}'. Usa tauri.register(...)`)
				);
			}
			// async → los throw del handler se convierten en rechazos (igual que Tauri)
			return Promise.resolve().then(() => fn(args));
		}
	};
});

vi.mock('@tauri-apps/api/core', () => ({
	invoke: (cmd: string, args?: Record<string, unknown>) => tauriState.invoke(cmd, args)
}));

// Puente hacia los tests (el objeto vive aquí; tauri.ts solo lo referencia)
interface TauriTestBridge {
	register(cmd: string, fn: (args?: any) => unknown): void;
	reset(): void;
}
(globalThis as unknown as { __tauriTestState?: TauriTestBridge }).__tauriTestState = tauriState;	afterEach(() => {
		cleanup();
		tauriState.reset();
		vi.clearAllMocks();
		// jsdom expone storage en el global; limpiar de forma defensiva
		try {
			window.localStorage?.clear();
			window.sessionStorage?.clear();
		} catch {
			/* entorno sin storage */
		}
	});
