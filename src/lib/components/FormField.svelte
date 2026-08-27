<script lang="ts">
	interface Props {
		label?: string;
		required?: boolean;
		hint?: string;
		error?: string;
		/** Espaciado vertical compacto (mb-3 en vez de mb-4). Útil en formularios densos. */
		dense?: boolean;
		/** Clases extra para el contenedor (ej. 'col-span-2'). */
		class?: string;
		children?: import('svelte').Snippet;
	}

	let {
		label,
		required = false,
		hint,
		error,
		dense = false,
		class: klass = '',
		children
	}: Props = $props();

	// ── Accesibilidad (WCAG 2.1 SC 1.3.1, 3.3.2, 4.1.2) ──
	// Generamos ids estables para (a) el input hijo y (b) el párrafo de hint/error,
	// y los conectamos vía `<label for>` + `aria-describedby` + `aria-invalid`.
	//
	// Como el input lo pasa el consumidor via snippet (no es hijo directo del label),
	// usamos un `$effect` que busca el primer control enfocable dentro del wrapper
	// y le inyecta los atributos ARIA necesarios. Esto evita tocar los ~30
	// consumidores existentes (progressive enhancement).
	//
	// El id del input solo se sobreescribe si el consumidor NO le puso uno propio
	// (para no romper selectores externos). En ese caso, el `<label for>` apunta al
	// id que el consumidor definió (se sincroniza vía `labelFor`).
	let fieldId = `ff-${Math.random().toString(36).slice(2, 10)}`;
	let descId = `${fieldId}-desc`;
	let wrapper: HTMLDivElement | undefined = $state();
	let labelFor = $state(fieldId);

	$effect(() => {
		if (!wrapper) return;
		// Busca el primer control de formulario dentro del wrapper.
		const input = wrapper.querySelector<HTMLElement>(
			'input, select, textarea, button[type="submit"]'
		);
		if (!input) return;
		// Si el consumidor ya le puso un id, lo respetamos y el `<label for>`
		// se actualiza para apuntar a ese id. Si no, le asignamos el nuestro.
		if (input.id) {
			labelFor = input.id;
		} else {
			input.id = fieldId;
			labelFor = fieldId;
		}
		// aria-describedby: vincula el input al párrafo de hint/error.
		if (error || hint) {
			input.setAttribute('aria-describedby', descId);
		} else {
			input.removeAttribute('aria-describedby');
		}
		// aria-invalid: marca el campo como erróneo para lectores de pantalla.
		if (error) {
			input.setAttribute('aria-invalid', 'true');
		} else {
			input.removeAttribute('aria-invalid');
		}
	});
</script>

<div bind:this={wrapper} class="{dense ? 'mb-3' : 'mb-4'} {klass}">
	{#if label}
		<label class="label" for={labelFor}>
			{label}
			{#if required}<span class="text-peligro" aria-hidden="true"> *</span>{/if}
		</label>
	{/if}
	{@render children?.()}
	{#if error}
		<p id={descId} class="mt-1 text-xs text-peligro flex items-center gap-1" aria-live="polite">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				class="w-3.5 h-3.5"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
				stroke-width="2"
				aria-hidden="true"
				><path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z"
				/></svg
			>
			{error}
		</p>
	{:else if hint}
		<p id={descId} class="mt-0.5 text-[10px] text-text-secondary/70 leading-tight">{hint}</p>
	{/if}
</div>
