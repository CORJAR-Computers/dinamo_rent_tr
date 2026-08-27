<script lang="ts">
	import FormField from './FormField.svelte';

	interface Props {
		label: string;
		/** Valor actual (controlado por el padre). */
		value: string;
		/** Opciones sugeridas (el valor actual se garantiza presente). */
		opciones: string[];
		required?: boolean;
		hint?: string;
		placeholder?: string;
		maxlength?: number;
		onchange: (v: string) => void;
	}

	let {
		label,
		value,
		opciones,
		required = false,
		hint,
		placeholder = '— Seleccionar —',
		maxlength = 100,
		onchange
	}: Props = $props();

	let modoNuevo = $state(false);
	let nuevoValor = $state('');
	let inputRef = $state<HTMLInputElement | null>(null);

	// Únicas, ordenadas y garantizando que el valor actual exista (para que el
	// select siempre refleje el estado aunque el valor venga de otra parte).
	const lista = $derived.by(() => {
		const set = new Set<string>();
		for (const o of opciones) {
			const t = o.trim();
			if (t) set.add(t);
		}
		const v = value.trim();
		if (v) set.add(v);
		return [...set].sort((a, b) => a.localeCompare(b, 'es'));
	});

	function onSelect(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		if (v === '__nuevo__') {
			modoNuevo = true;
			nuevoValor = value;
			requestAnimationFrame(() => inputRef?.focus());
			return;
		}
		if (v !== value) onchange(v);
	}

	function confirmarNuevo() {
		const t = nuevoValor.trim();
		modoNuevo = false;
		if (t && t !== value) onchange(t);
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			confirmarNuevo();
		} else if (e.key === 'Escape') {
			modoNuevo = false;
		}
	}
</script>

<FormField {label} {required} {hint}>
	{#if modoNuevo}
		<div class="flex gap-2">
			<input
				bind:this={inputRef}
				class="input grow"
				bind:value={nuevoValor}
				placeholder="Escribir y presionar Enter…"
				{maxlength}
				onkeydown={onKeydown}
				onblur={confirmarNuevo}
			/>
			<button
				type="button"
				class="btn-outline !px-3 shrink-0"
				onclick={confirmarNuevo}
				title="Agregar"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="w-4 h-4"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="2"
					><path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" /></svg
				>
			</button>
		</div>
	{:else}
		<select class="input" {value} onchange={onSelect}>
			<option value="">{placeholder}</option>
			{#each lista as o}
				<option value={o}>{o}</option>
			{/each}
			<option value="__nuevo__">＋ Agregar nuevo…</option>
		</select>
	{/if}
</FormField>
