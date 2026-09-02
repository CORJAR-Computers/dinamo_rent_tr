<script lang="ts">
	import Modal from '$lib/components/Modal.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import type { RentaCierreDatos } from '$lib/api';

	interface Props {
		open: boolean;
		rentaId: number | null;
		cierre: RentaCierreDatos;
		cierreError: string;
		cerrando: boolean;
		nivelTanqueList: string[];
		onCalcular: () => void;
		onConfirmar: () => void;
		onClose: () => void;
	}

	let {
		open,
		rentaId,
		cierre = $bindable(),
		cierreError,
		cerrando,
		nivelTanqueList,
		onCalcular,
		onConfirmar,
		onClose
	}: Props = $props();
</script>

<Modal
	{open}
	title={rentaId !== null ? `Cerrar renta #${rentaId}` : ''}
	subtitle="Registra la devolución real; el sistema recalcula los totales."
	{onClose}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if cierreError}
			<div
				class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro"
				role="alert"
			>
				{cierreError}
			</div>
		{/if}
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Fecha de devolución real" required>
				<input
					class="input"
					type="date"
					bind:value={cierre.fechaDevolucionReal}
					onchange={onCalcular}
				/>
			</FormField>
			<FormField label="Hora de devolución" hint="Al cambiar se recalculan días/horas">
				<input
					class="input"
					type="time"
					bind:value={cierre.horaDevolucionReal}
					onchange={onCalcular}
				/>
			</FormField>
			<FormField label="Km final">
				<input
					class="input"
					inputmode="numeric"
					placeholder="Km al devolver"
					bind:value={cierre.kmFinal}
				/>
			</FormField>
			<FormField label="Tanque final">
				<select class="input" bind:value={cierre.tanqueFinal}>
					{#each nivelTanqueList as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField
				label="Días cobrados"
				hint="Auto desde la devolución real (excedente > 3 h = día completo)."
			>
				<input
					class="input"
					type="number"
					min="0"
					step="1"
					placeholder="Mantener"
					bind:value={cierre.diasCalculados}
				/>
			</FormField>
			<FormField label="Horas extras finales" hint="Excedente ≤ 3 h, redondeadas hacia arriba.">
				<input
					class="input"
					type="number"
					min="0"
					step="1"
					placeholder="Mantener"
					bind:value={cierre.horasExtras}
				/>
			</FormField>
			<FormField label="Valor día final (COP)">
				<input
					class="input"
					inputmode="decimal"
					placeholder="Mantener"
					bind:value={cierre.valorDia}
				/>
			</FormField>
			<FormField label="Valor hora extra final (COP)">
				<input
					class="input"
					inputmode="decimal"
					placeholder="Mantener"
					bind:value={cierre.valorHoraExtra}
				/>
			</FormField>
			<FormField label="Valor días extra final (COP)" hint="Días adicionales no contemplados en el plazo">
				<input
					class="input"
					inputmode="decimal"
					placeholder="Mantener"
					bind:value={cierre.valorDiaExtra}
				/>
			</FormField>
			<FormField label="Descuento final (COP)">
				<input
					class="input"
					inputmode="decimal"
					placeholder="Mantener"
					bind:value={cierre.descuento}
				/>
			</FormField>
			<div class="col-span-full mb-1">
				<label
					class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors w-fit"
				>
					<input
						type="checkbox"
						class="accent-primary"
						bind:checked={cierre.cobrarHorasExtra}
						onchange={onCalcular}
					/>
					Cobrar horas extras
					<span class="text-xs text-text-secondary">(desmarcar si aplica hora de gracia o cortesía)</span>
				</label>
			</div>
			<div class="col-span-full">
				<FormField label="Observaciones de la devolución">
					<textarea
						class="input min-h-17.5 resize-y"
						bind:value={cierre.observaciones}
						maxlength="2000"
					></textarea>
				</FormField>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onClose} disabled={cerrando}>Cancelar</button>
		<button class="btn-primary" onclick={onConfirmar} disabled={cerrando}>
			{#if cerrando}
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
				Cerrando...
			{:else}
				Cerrar renta
			{/if}
		</button>
	{/snippet}
</Modal>
