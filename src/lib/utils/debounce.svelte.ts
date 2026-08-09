// debounce.svelte.ts — Hooks de debounce reutilizables (Svelte 5 runes)
//
// Runes ($state / $effect) son legales en archivos `.svelte.ts` desde Svelte 5.
// Los hooks deben invocarse desde el contexto de un componente (script de .svelte
// o desde otro hook invocado a su vez por un componente). `onDestroy` se registra
// contra el ciclo de vida del componente que llama.

import { onDestroy } from 'svelte';

/**
 * Hook de búsqueda con debounce. Devuelve un objeto con un getter reactivo
 * `debounced` que se actualiza `delay` ms después de que `getter()` deja de
 * cambiar. Cancela el timer al desmontar el componente.
 *
 * @example
 * ```ts
 * const busquedaDebounced = useDebouncedSearch(() => busqueda, 350);
 * $effect(() => {
 *   const _ = busquedaDebounced.debounced; // suscripción reactiva
 *   cargar(); // se ejecuta cuando `debounced` cambia
 * });
 * ```
 *
 * Nota: el tipo de retorno es `{ debounced: T }` (getter), pero la
 * implementación usa `get debounced()` para mantener la reactividad del
 * `$state` interno al acceder desde el consumidor.
 */
export function useDebouncedSearch<T>(getter: () => T, delay = 350): { debounced: T } {
	let debounced = $state<T>(getter()) as T;
	let timer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		const v = getter();
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => {
			debounced = v;
		}, delay);
	});

	onDestroy(() => {
		if (timer) clearTimeout(timer);
	});

	return { get debounced() { return debounced; } };
}

/**
 * Hook para debounce de una función de recarga (`fn`). Reemplaza el patrón
 * manual `let searchTimer; let primerCiclo = true; $effect(() => { ... })`
 * repetido en 9 rutas.
 *
 * Encapsula:
 *  - el flag `primerCiclo` (primera invocación tras montaje)
 *  - el timer de debounce (con `clearTimeout` al reprogramar)
 *  - el cleanup al desmontar (`onDestroy`)
 *
 * @param fn            Función a ejecutar tras el delay (p.ej. `cargar`).
 * @param opts.delay    Ms de debounce. Default 350.
 * @param opts.skipFirst Si true, omite la primera invocación (caso `rentas`,
 *                       donde la carga inicial la hace `onMount`). Default false.
 * @param opts.immediateIf Si devuelve true, ejecuta `fn` sin debounce (caso
 *                       típico: `() => !busqueda.trim()` para recargar al vaciar
 *                       la búsqueda).
 * @returns `schedule` Función a llamar desde dentro de un `$effect` (que ya
 *                       esté suscrito a las señales que interesan).
 *
 * @example
 * ```ts
 * const scheduleReload = useDebouncedEffect(cargar, {
 *   skipFirst: true,
 *   immediateIf: () => !busqueda.trim()
 * });
 * $effect(() => {
 *   const _ = busqueda;        // suscripción
 *   const _e = estadoFiltro;   // suscripción
 *   const _p = placaFiltro;    // suscripción
 *   scheduleReload();
 * });
 * ```
 */
export function useDebouncedEffect(
	fn: () => void,
	opts: { delay?: number; skipFirst?: boolean; immediateIf?: () => boolean } = {}
): () => void {
	const { delay = 350, skipFirst = false, immediateIf } = opts;
	let primerCiclo = true;
	let timer: ReturnType<typeof setTimeout> | undefined;

	onDestroy(() => {
		if (timer) clearTimeout(timer);
	});

	return () => {
		if (primerCiclo) {
			primerCiclo = false;
			if (skipFirst) return;
		}
		if (timer) clearTimeout(timer);
		if (immediateIf?.()) {
			fn();
			return;
		}
		timer = setTimeout(fn, delay);
	};
}
