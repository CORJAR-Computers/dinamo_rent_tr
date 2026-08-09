<script lang="ts">
	import Icon from './Icon.svelte';

	/** Resultado formateado para la lista del panel. */
	export interface ResultadoCopiar {
		id: string;
		titulo: string;
		subtitulo: string;
		/** Si es true, la fila se muestra deshabilitada (no se puede duplicar). */
		bloqueado?: boolean;
		/** Tooltip cuando la fila está bloqueada. */
		razonBloqueo?: string;
		/** Payload que se entrega a `onSeleccionar`. */
		datos: unknown;
	}

	interface Props {
		/** Solo se activa (búsqueda y atajo Ctrl+Shift+C) cuando es true. */
		activo: boolean;
		/** Texto del botón del panel. */
		titulo: string;
		/** Placeholder del buscador. */
		placeholder: string;
		/** Busca en el servidor y devuelve los resultados formateados. */
		buscar: (termino: string) => Promise<ResultadoCopiar[]>;
		/** Se llama con `datos` del resultado elegido (el padre rellena su formulario). */
		onSeleccionar: (datos: unknown) => void;
		/** Frase que sigue a «Datos copiados de X.» al seleccionar. */
		notaPaso?: string;
	}

	let {
		activo,
		titulo,
		placeholder,
		buscar,
		onSeleccionar,
		notaPaso = 'Ajusta los campos únicos antes de guardar.'
	}: Props = $props();

	let panelCopiar = $state(false);
	let termCopiar = $state('');
	let resultadosCopiar = $state<ResultadoCopiar[]>([]);
	let buscandoCopiar = $state(false);
	let copiadoDe = $state('');
	let copiarError = $state('');
	let copiarTimer: ReturnType<typeof setTimeout> | undefined;
	let copiarReq = 0; // token anti-race: solo la búsqueda más reciente actualiza el estado
	let copiarSearchInput = $state<HTMLInputElement | undefined>(undefined);

	// Resultados visibles solo cuando hay un término de búsqueda válido
	const resultadosVisibles = $derived(termCopiar.trim().length >= 2 ? resultadosCopiar : []);

	// Reset del panel cada vez que se reactiva (apertura del modal en modo crear)
	$effect(() => {
		if (activo) {
			clearTimeout(copiarTimer);
			panelCopiar = false;
			termCopiar = '';
			resultadosCopiar = [];
			buscandoCopiar = false;
			copiadoDe = '';
			copiarError = '';
		}
	});

	// Ctrl+Shift+C: abre (o cierra) el panel y enfoca el buscador.
	function onGlobalKeydown(e: KeyboardEvent) {
		if (e.key.toLowerCase() !== 'c' || !e.ctrlKey || !e.shiftKey || e.altKey || e.metaKey) return;
		if (!activo) return;
		e.preventDefault();
		togglePanelCopiar();
	}

	function togglePanelCopiar() {
		panelCopiar = !panelCopiar;
		if (panelCopiar) requestAnimationFrame(() => copiarSearchInput?.focus());
	}

	// Búsqueda en el servidor. `copiarReq` evita que una respuesta vieja pise
	// una búsqueda más reciente.
	async function buscarResultados() {
		const term = termCopiar.trim();
		const req = ++copiarReq;
		if (term.length < 2) {
			resultadosCopiar = [];
			buscandoCopiar = false;
			return;
		}
		buscandoCopiar = true;
		try {
			const r = await buscar(term);
			if (req !== copiarReq) return; // respuesta obsoleta
			resultadosCopiar = r;
			copiarError = '';
		} catch {
			if (req === copiarReq) {
				resultadosCopiar = [];
				copiarError = 'No se pudo buscar. Intenta de nuevo.';
			}
		} finally {
			if (req === copiarReq) buscandoCopiar = false;
		}
	}

	function onTermCopiar(v: string) {
		termCopiar = v;
		copiadoDe = ''; // una búsqueda nueva invalida el aviso del duplicado anterior
		copiarError = '';
		clearTimeout(copiarTimer);
		copiarTimer = setTimeout(buscarResultados, 300);
	}

	/** Entrega el resultado al padre y limpia el panel (el aviso queda visible). */
	function seleccionar(r: ResultadoCopiar) {
		copiadoDe = r.titulo;
		termCopiar = '';
		resultadosCopiar = [];
		buscandoCopiar = false;
		onSeleccionar(r.datos);
	}
</script>

<!-- Ctrl+Shift+C abre/cierra el panel (el componente solo está montado con el modal abierto en modo crear) -->
<svelte:window onkeydown={onGlobalKeydown} />

<div class="mb-4 rounded-xl border border-primary/20 bg-primary/5 overflow-hidden">
	<button
		type="button"
		class="w-full flex items-center justify-between gap-2 px-3 py-2.5 text-sm font-semibold text-primary hover:bg-primary/10 transition-colors"
		onclick={() => (panelCopiar = !panelCopiar)}
		aria-expanded={panelCopiar}
	>
		<span class="inline-flex items-center gap-2">
			<Icon name="clipboard" class="w-4 h-4" />
			{titulo}
		</span>
		<kbd class="hidden sm:inline-flex items-center rounded border border-primary/25 bg-primary/10 px-1.5 py-0.5 text-[10px] font-mono text-primary/80 leading-none">Ctrl+Shift+C</kbd>
		<svg
			xmlns="http://www.w3.org/2000/svg"
			class="w-4 h-4 shrink-0 transition-transform {panelCopiar ? 'rotate-180' : ''}"
			fill="none"
			viewBox="0 0 24 24"
			stroke="currentColor"
			stroke-width="2"
		><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" /></svg>
	</button>

	{#if panelCopiar}
		<div class="px-3 pb-3 pt-2.5 border-t border-primary/10">
			<div class="relative">
				<Icon name="search" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" />
				<input
					class="input pl-9"
					type="search"
					{placeholder}
					bind:this={copiarSearchInput}
					value={termCopiar}
					oninput={(e) => onTermCopiar((e.currentTarget as HTMLInputElement).value)}
					onkeydown={(e) => {
						if (e.key === 'Escape') {
							termCopiar = '';
							resultadosCopiar = [];
						}
					}}
				/>
			</div>

			{#if copiadoDe}
				<p class="mt-2 text-xs text-exito flex items-start gap-1.5">
					<Icon name="check" class="w-3.5 h-3.5 shrink-0 mt-0.5" />
					<span>
						Datos copiados de <strong>{copiadoDe}</strong>. {notaPaso}
					</span>
				</p>
			{/if}

			{#if termCopiar.trim() && termCopiar.trim().length < 2}
				<p class="mt-1.5 text-xs text-text-secondary">Escribe al menos 2 caracteres para buscar.</p>
			{/if}

			{#if buscandoCopiar}
				<p class="mt-2 text-xs text-text-secondary inline-flex items-center gap-1.5">
					<svg class="animate-spin h-3.5 w-3.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
					Buscando…
				</p>
			{/if}

			{#if resultadosVisibles.length > 0}
				<ul class="mt-2 max-h-52 overflow-y-auto rounded-lg border border-border bg-surface divide-y divide-border/60">
					{#each resultadosVisibles as r (r.id)}
						<li>
							<button
								type="button"
								class="w-full text-left px-3 py-2.5 flex items-center gap-3 transition-colors hover:bg-primary/10 disabled:opacity-45 disabled:hover:bg-transparent disabled:cursor-not-allowed"
								disabled={r.bloqueado}
								title={r.bloqueado ? r.razonBloqueo ?? 'No se pueden copiar los datos.' : `Copiar datos de ${r.titulo}`}
								onclick={() => seleccionar(r)}
							>
								<div class="grow min-w-0">
									<p class="text-sm font-semibold text-text-primary truncate">{r.titulo}</p>
									<p class="text-xs text-text-secondary truncate">{r.subtitulo}</p>
								</div>
								{#if r.bloqueado}
									<Icon name="lock" class="w-4 h-4 shrink-0 text-text-secondary/50" />
								{:else}
									<Icon name="clipboard" class="w-4 h-4 shrink-0 text-primary/70" />
								{/if}
							</button>
						</li>
					{/each}
				</ul>
			{:else if copiarError}
				<p class="mt-1.5 text-xs text-peligro" role="alert">{copiarError}</p>
			{:else if termCopiar.trim().length >= 2 && !buscandoCopiar}
				<p class="mt-1.5 text-xs text-text-secondary">Sin resultados para «{termCopiar.trim()}».</p>
			{/if}
		</div>
	{/if}
</div>
