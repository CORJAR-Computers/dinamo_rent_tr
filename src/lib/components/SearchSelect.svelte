<script lang="ts">
	import { onMount } from 'svelte';
	import FormField from './FormField.svelte';

	export interface SearchSelectOpcion {
		/** Valor que se devuelve al seleccionar (ej. id o placa). */
		value: string;
		/** Texto principal visible. */
		label: string;
		/** Texto secundario que también se filtra (ej. número de documento). */
		sub?: string;
	}

	interface Props {
		label?: string;
		hint?: string;
		required?: boolean;
		dense?: boolean;
		/** Clases extra para el contenedor (ej. 'col-span-2'). */
		class?: string;
		/** Valor actual (controlado por el padre). */
		value: string;
		opciones: SearchSelectOpcion[];
		onchange: (v: string) => void;
		placeholder?: string;
		/** Etiqueta de la opción vacía (para deseleccionar). */
		vacioLabel?: string;
		disabled?: boolean;
		/** Máximo de coincidencias mostradas antes de pedir más filtro. */
		max?: number;
	}

	let {
		label,
		hint,
		required = false,
		dense = false,
		class: klass = '',
		value,
		opciones,
		onchange,
		placeholder = 'Escribir para buscar…',
		vacioLabel = '— Sin seleccionar —',
		disabled = false,
		max = 100
	}: Props = $props();

	let abierto = $state(false);
	let query = $state('');
	let resaltado = $state(-1);
	let rootRef = $state<HTMLDivElement | null>(null);
	let inputRef = $state<HTMLInputElement | null>(null);
	let listaRef = $state<HTMLUListElement | null>(null);

	// Normaliza mayúsculas y tildes para que «JOSE» y «José» coincidan.
	const norm = (s: string) => s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');

	const labelActual = $derived(opciones.find((o) => o.value === value)?.label ?? '');

	const filtradas = $derived.by(() => {
		const q = norm(query.trim());
		const base = q
			? opciones.filter(
					(o) => norm(o.label).includes(q) || (o.sub ? norm(o.sub).includes(q) : false)
				)
			: opciones;
		return base.slice(0, max);
	});

	const hayMas = $derived(opciones.length > filtradas.length);

	function abrir() {
		if (disabled) return;
		abierto = true;
		query = '';
		resaltado = -1;
		// Al enfocar por tab/click el cursor queda al final para escribir de inmediato.
		requestAnimationFrame(() => {
			inputRef?.focus();
			inputRef?.setSelectionRange(inputRef.value.length, inputRef.value.length);
		});
	}

	function cerrar() {
		abierto = false;
		query = '';
		resaltado = -1;
	}

	function seleccionar(v: string) {
		cerrar();
		if (v !== value) onchange(v);
	}

	// Los <li role="option"> responden a Enter/Espacio si algún día reciben
	// foco (el teclado principal lo maneja el input combobox con flechas).
	function onOpcionKeydown(e: KeyboardEvent, v: string) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			seleccionar(v);
		}
	}

	function onInput() {
		if (!abierto) abierto = true;
		query = inputRef?.value ?? '';
		resaltado = -1;
	}

	function onKeydown(e: KeyboardEvent) {
		if (!abierto && (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ')) {
			e.preventDefault();
			abrir();
			return;
		}
		if (!abierto) return;
		if (e.key === 'Escape') {
			e.preventDefault();
			cerrar();
			return;
		}
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			resaltado = Math.min(resaltado + 1, filtradas.length - 1);
			return;
		}
		if (e.key === 'ArrowUp') {
			e.preventDefault();
			resaltado = Math.max(resaltado - 1, 0);
			return;
		}
		if (e.key === 'Enter') {
			e.preventDefault();
			const sel = resaltado >= 0 ? filtradas[resaltado] : filtradas[0];
			if (sel) seleccionar(sel.value);
			return;
		}
		if (e.key === 'Tab' && abierto) {
			cerrar();
		}
	}

	function onBlurInput() {
		// Pequeño retardo para que el clic en la lista tenga tiempo de dispararse
		// antes de cerrar (el pointerdown global ya cierra al hacer clic fuera).
		setTimeout(() => {
			if (abierto) cerrar();
		}, 120);
	}

	onMount(() => {
		function onDocPointer(e: PointerEvent) {
			if (abierto && rootRef && !rootRef.contains(e.target as Node)) cerrar();
		}
		document.addEventListener('pointerdown', onDocPointer);
		return () => document.removeEventListener('pointerdown', onDocPointer);
	});

	// Mantiene visible la opción resaltada con el teclado.
	$effect(() => {
		if (abierto && resaltado >= 0 && listaRef) {
			const el = listaRef.querySelector(`[data-idx="${resaltado}"]`);
			el?.scrollIntoView?.({ block: 'nearest' });
		}
	});
</script>

<FormField label={label ?? ''} {hint} {required} {dense} class={klass}>
	<div bind:this={rootRef} class="relative">
		<div class="relative">
			<input
				bind:this={inputRef}
				class="input pr-8 {abierto ? 'rounded-b-none border-b-transparent' : ''} {disabled ? 'opacity-60 cursor-not-allowed' : ''}"
				type="text"
				role="combobox"
				aria-label={label || 'Buscar y seleccionar'}
				aria-expanded={abierto}
				aria-controls={abierto ? 'lista-opciones' : undefined}
				aria-activedescendant={abierto && resaltado >= 0 ? `opt-${resaltado}` : undefined}
				placeholder={placeholder}
				value={abierto ? query : labelActual}
				oninput={onInput}
				onfocus={abrir}
				onkeydown={onKeydown}
				onblur={onBlurInput}
				disabled={disabled}
				autocomplete="off"
				spellcheck="false"
			/>
			<svg
				class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-secondary/60 transition-transform {abierto ? 'rotate-180' : ''}"
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
				stroke-width="2"
			><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" /></svg>
		</div>

		{#if abierto}
			<ul
				bind:this={listaRef}
				id="lista-opciones"
				role="listbox"
				class="absolute z-30 mt-1 w-full max-h-64 overflow-auto rounded-lg border border-border bg-surface shadow-lg"
			>
				<li
					role="option"
					tabindex="-1"
					data-idx="-1"
					aria-selected={value === ''}
					class="px-3 py-2 text-sm cursor-pointer hover:bg-primary/10 {value === ''
						? 'bg-primary/5 text-primary font-semibold'
						: 'text-text-secondary'}"
					onclick={() => seleccionar('')}
					onkeydown={(e) => onOpcionKeydown(e, '')}
					onmouseenter={() => (resaltado = -1)}
				>
					{vacioLabel}
				</li>
				{#each filtradas as o, i}
					<li
						role="option"
						tabindex="-1"
						id={`opt-${i}`}
						data-idx={i}
						aria-selected={o.value === value}
						class="px-3 py-2 text-sm cursor-pointer {i === resaltado
							? 'bg-primary/15'
							: 'hover:bg-primary/10'} {o.value === value
							? 'text-primary font-semibold'
							: 'text-text-primary'}"
						onclick={() => seleccionar(o.value)}
						onkeydown={(e) => onOpcionKeydown(e, o.value)}
						onmouseenter={() => (resaltado = i)}
					>
						<span class="block truncate">{o.label}</span>
						{#if o.sub}
							<span class="block text-[11px] text-text-secondary truncate">{o.sub}</span>
						{/if}
					</li>
				{/each}
				{#if filtradas.length === 0}
					<li class="px-3 py-2 text-sm text-text-secondary italic">
						Sin coincidencias para «{query}»
					</li>
				{/if}
				{#if hayMas}
					<li class="px-3 py-1.5 text-[11px] text-text-secondary border-t border-border">
						{opciones.length - filtradas.length} más — sigue escribiendo para filtrar
					</li>
				{/if}
			</ul>
		{/if}
	</div>
</FormField>
