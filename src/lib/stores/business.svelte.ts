// business.svelte.ts — Store global de BusinessLists con TTL (Svelte 5 runes)
//
// TAREA 3.2 (Bloque 3 — Performance): evita que cada ruta (rentas, autos,
// clientes, reservas, gastos, mantenimiento, comparendos, usuarios…) llame a
// `businessApi.listas(sid)` en su `onMount`. El store cachea el resultado con
// un TTL configurable (por defecto 5 min) y lo sirve desde memoria a todas las
// rutas. La primera ruta que monta dispara la carga; las siguientes leen del
// cache sin round-trip.
//
// Patrón de uso en una ruta:
//
//   import { businessLists } from '$lib/stores/business.svelte';
//
//   onMount(async () => {
//       // Carga las listas si no están en cache (o si expiró el TTL).
//       // `lists` es reactivo: el template puede usarlo directamente.
//       await businessLists.ensure(sid());
//       // …cargar otros datos de la ruta…
//   });
//
//   // En el template:
//   $derived(businessLists.lists?.tiposAuto ?? [])
//
// Invalidación: cuando una ruta modifica datos que afectan las listas (p. ej.
// empresa.config actualiza `impuestoPorcentaje`), debe llamar a
// `businessLists.invalidate()` para que la próxima lectura vuelva a consultar
// el backend.

import { businessApi, type BusinessLists } from '$lib/api';

/** TTL por defecto: 5 minutos (300_000 ms). */
const DEFAULT_TTL_MS = 5 * 60 * 1000;

class BusinessListsStore {
	/** Listas cacheadas (reactivo: el template puede leer directamente). */
	lists = $state<BusinessLists | null>(null);

	/** Timestamp de la última carga exitosa (ms desde epoch). */
	private cargadoEn = $state<number>(0);

	/** TTL aplicado a este store (ms). Modificable en caliente. */
	ttlMs = $state(DEFAULT_TTL_MS);

	/** Indica si hay una carga en vuelo (para no duplicar requests). */
	private cargando: Promise<BusinessLists | null> | null = null;

	/** ¿El cache es válido (no expirado y con datos)? */
	get valido(): boolean {
		return this.lists !== null && Date.now() - this.cargadoEn < this.ttlMs;
	}

	/**
	 * Devuelve las listas desde el cache si es válido; si no, dispara una
	 * carga (sin await: la promesa resuelve con las listas cuando termine,
	 * pero `this.lists` se actualiza reactivamente para que el template
	 * muestre el spinner por defecto y se re-renderice al llegar).
	 *
	 * Si hay una carga en vuelo, reutiliza la misma promesa (deduplica).
	 */
	async ensure(sessionId: string): Promise<BusinessLists | null> {
		if (this.valido) return this.lists;
		if (!sessionId) return null;
		if (this.cargando) return this.cargando;

		this.cargando = (async () => {
			try {
				const data = await businessApi.listas(sessionId);
				this.lists = data;
				this.cargadoEn = Date.now();
				return data;
			} catch (e) {
				console.warn('businessLists: no se pudieron cargar las listas:', e);
				return null;
			} finally {
				this.cargando = null;
			}
		})();
		return this.cargando;
	}

	/**
	 * Fuerza la recarga en la próxima lectura (o inmediatamente si se pasa
	 * `sessionId`). Las rutas que modifican datos del backend que afectan
	 * las listas (impuestoPorcentaje, roles_con_*, etc.) deben llamar a
	 * este método tras confirmar el cambio.
	 */
	invalidate(sessionId?: string): void {
		this.cargadoEn = 0;
		if (sessionId) {
			void this.ensure(sessionId);
		}
	}

	/** Resetea el store (logout). */
	clear(): void {
		this.lists = null;
		this.cargadoEn = 0;
		this.cargando = null;
	}
}

export const businessLists = new BusinessListsStore();
