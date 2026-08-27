<script lang="ts">
	import Modal from '$lib/components/Modal.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import type { InspeccionDatos } from '$lib/api';

	interface Props {
		open: boolean;
		rentaId: number | null;
		inspeccionTipo: 'Salida' | 'Entrada';
		inspeccion: InspeccionDatos;
		inspeccionError: string;
		guardandoInspeccion: boolean;
		nivelTanqueList: string[];
		onTipoChange: (tipo: 'Salida' | 'Entrada') => void;
		onConfirmar: () => void;
		onClose: () => void;
	}

	let {
		open,
		rentaId,
		inspeccionTipo = $bindable(),
		inspeccion = $bindable(),
		inspeccionError,
		guardandoInspeccion,
		nivelTanqueList,
		onTipoChange,
		onConfirmar,
		onClose
	}: Props = $props();
</script>

<Modal
	{open}
	title={rentaId !== null ? `Inspección de ${inspeccionTipo} — renta #${rentaId}` : ''}
	subtitle="Verificación del estado del vehículo al entregar o recibir."
	{onClose}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if inspeccionError}
			<div
				class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro"
				role="alert"
			>
				{inspeccionError}
			</div>
		{/if}
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<div class="col-span-full mb-1">
				<div
					class="inline-flex rounded-lg border border-border p-1 bg-alt-row/60"
					role="tablist"
					aria-label="Tipo de inspección"
				>
					{#each ['Salida', 'Entrada'] as t}
						<button
							type="button"
							class="px-3 py-1.5 rounded-md text-sm font-semibold transition-colors {inspeccionTipo ===
							t
								? 'bg-primary text-white shadow'
								: 'text-text-secondary hover:text-text-primary'}"
							role="tab"
							aria-selected={inspeccionTipo === t}
							onclick={() => onTipoChange(t as 'Salida' | 'Entrada')}
						>
							{t}
						</button>
					{/each}
				</div>
			</div>
			<FormField label="Kilometraje" required>
				<input
					class="input"
					inputmode="numeric"
					placeholder="Km actual"
					bind:value={inspeccion.kilometraje}
				/>
			</FormField>
			<FormField label="Nivel de gasolina" required>
				<select class="input" bind:value={inspeccion.nivelGasolina}>
					{#each nivelTanqueList as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Limpieza">
				<select class="input" bind:value={inspeccion.limpieza}>
					{#each ['Limpio', 'Aceptable', 'Sucio'] as l}
						<option value={l}>{l}</option>
					{/each}
				</select>
			</FormField>
			<div class="col-span-full grid grid-cols-2 sm:grid-cols-4 gap-2">
				<label
					class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors"
				>
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneRepuesto} />
					Llanta repuesto
				</label>
				<label
					class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors"
				>
					<input
						type="checkbox"
						class="accent-primary"
						bind:checked={inspeccion.tieneGatoCruceta}
					/>
					Gato / cruceta
				</label>
				<label
					class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors"
				>
					<input
						type="checkbox"
						class="accent-primary"
						bind:checked={inspeccion.tieneKitCarretera}
					/>
					Kit carretera
				</label>
				<label
					class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors"
				>
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneDocumentos} />
					Documentos
				</label>
			</div>
			<FormField label="Daños de carrocería">
				<textarea
					class="input min-h-15 resize-y"
					placeholder="Describir golpes, rayones..."
					bind:value={inspeccion.danosCarroceria}
					maxlength="2000"
				></textarea>
			</FormField>
			<FormField label="Observaciones">
				<textarea
					class="input min-h-15 resize-y"
					bind:value={inspeccion.observaciones}
					maxlength="2000"
				></textarea>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onClose} disabled={guardandoInspeccion}>Cancelar</button>
		<button class="btn-primary" onclick={onConfirmar} disabled={guardandoInspeccion}>
			{#if guardandoInspeccion}
				<svg
					class="animate-spin h-4 w-4"
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
					></circle><path
						class="opacity-75"
						fill="currentColor"
						d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
					></path></svg
				>
				Guardando...
			{:else}
				Registrar inspección
			{/if}
		</button>
	{/snippet}
</Modal>
