<script lang="ts">
	import Modal from '$lib/components/Modal.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import { formatCOP } from '$lib/utils/format';
	import type { RentaCierreEditDatos, Renta } from '$lib/api';

	interface Props {
		open: boolean;
		renta: Renta | null;
		editCerrada: RentaCierreEditDatos;
		editCerradaError: string;
		editandoCerrada: boolean;
		onConfirmar: () => void;
		onClose: () => void;
	}

	let {
		open,
		renta,
		editCerrada = $bindable(),
		editCerradaError,
		editandoCerrada,
		onConfirmar,
		onClose
	}: Props = $props();
</script>

<Modal
	{open}
	title={renta ? `Corregir renta cerrada #${String(renta.id).padStart(4, '0')}` : ''}
	subtitle="Modifica los campos financieros y recalcula los totales."
	{onClose}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if editCerradaError}
			<div
				class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro"
				role="alert"
			>
				{editCerradaError}
			</div>
		{/if}

		<div
			class="mb-4 rounded-lg bg-alerta/10 border border-alerta/30 px-3 py-2.5 text-sm text-alerta"
		>
			<strong>⚠️ Atención:</strong> Solo los campos financieros se modificarán. El abono, el cliente y
			la placa NO se pueden editar.
		</div>

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Valor día" required>
				<input class="input" type="number" step="0.01" min="0" bind:value={editCerrada.valorDia} />
			</FormField>
			<FormField label="Valor hora extra">
				<input
					class="input"
					type="number"
					step="0.01"
					min="0"
					bind:value={editCerrada.valorHoraExtra}
				/>
			</FormField>
			<FormField label="Días calculados" required>
				<input class="input" type="number" min="1" bind:value={editCerrada.diasCalculados} />
			</FormField>
			<FormField label="Horas extras">
				<input class="input" type="number" min="0" bind:value={editCerrada.horasExtras} />
			</FormField>
			<FormField label="Descuento">
				<input class="input" type="number" step="0.01" min="0" bind:value={editCerrada.descuento} />
			</FormField>
			<div class="col-span-full">
				<FormField label="Motivo de la corrección" required hint="Obligatorio para auditoría">
					<textarea
						class="input min-h-15 resize-y"
						placeholder="Describe el error de digitación que se corrige..."
						bind:value={editCerrada.observaciones}
						maxlength="500"
					></textarea>
				</FormField>
			</div>

			<div class="mt-4 p-3 rounded-lg bg-alt-row/60 border border-border">
				<p class="text-sm font-semibold text-text-primary mb-2">Valores actuales de la renta:</p>
				<p class="text-sm text-text-secondary">
					Total: <span class="font-semibold text-text-primary"
						>{formatCOP(renta?.total ?? '0')}</span
					>
					| Saldo:
					<span class="font-semibold">{formatCOP(renta?.saldoPendiente ?? '0')}</span>
				</p>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onClose} disabled={editandoCerrada}>Cancelar</button>
		<button class="btn-primary" onclick={onConfirmar} disabled={editandoCerrada}>
			{#if editandoCerrada}
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
				Aplicar corrección
			{/if}
		</button>
	{/snippet}
</Modal>
