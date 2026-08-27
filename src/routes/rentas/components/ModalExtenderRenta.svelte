<script lang="ts">
	import Modal from '$lib/components/Modal.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import type { ExtensionDatos, ExtensionRenta, Renta } from '$lib/api';

	interface Props {
		open: boolean;
		renta: Renta | null;
		extension: ExtensionDatos;
		extenderError: string;
		extenderando: boolean;
		historialExtensiones: ExtensionRenta[];
		cargandoHistorial: boolean;
		fmtHora: (h: string | null) => string;
		onConfirmar: () => void;
		onClose: () => void;
	}

	let {
		open,
		renta,
		extension = $bindable(),
		extenderError,
		extenderando,
		historialExtensiones,
		cargandoHistorial,
		fmtHora,
		onConfirmar,
		onClose
	}: Props = $props();
</script>

<Modal
	{open}
	title={renta ? `Extender renta #${String(renta.id).padStart(4, '0')}` : ''}
	subtitle="Agregar horas o días extras a la renta activa."
	{onClose}
	width="max-w-md"
>
	{#snippet children()}
		{#if extenderError}
			<div
				class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro"
				role="alert"
			>
				{extenderError}
			</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Tipo de extensión" required>
				<select class="input" bind:value={extension.tipo}>
					<option value="horas">Horas extra</option>
					<option value="dias">Día(s) extra</option>
				</select>
			</FormField>
			<FormField label="Cantidad" required>
				<input class="input" type="number" min="1" bind:value={extension.cantidad} />
			</FormField>
			<FormField
				label="Valor unitario"
				required
				hint={extension.tipo === 'horas' ? 'Valor por hora extra' : 'Valor por día extra'}
			>
				<input
					class="input"
					type="number"
					step="0.01"
					min="0"
					placeholder="$0"
					bind:value={extension.valor}
				/>
			</FormField>
			<FormField label="Observaciones">
				<input
					class="input"
					placeholder="Motivo de la extensión..."
					bind:value={extension.observaciones}
					maxlength="200"
				/>
			</FormField>
		</div>

		{#if renta}
			<div class="mt-4 p-3 rounded-lg bg-alt-row/60 border border-border">
				<p class="text-sm font-semibold text-text-primary mb-2">Resumen:</p>
				<div class="text-sm text-text-secondary space-y-1">
					<p>
						Retorno actual: <span class="font-semibold"
							>{formatDate(renta.fechaRetorno)} {fmtHora(renta.horaRetorno)}</span
						>
					</p>
					<p>
						Nuevo retorno: <span class="font-semibold text-exito">
							{extension.tipo === 'horas'
								? `${extension.cantidad} hora(s) más`
								: `${extension.cantidad} día(s) más`}
						</span>
					</p>
					{#if extension.valor && parseFloat(extension.valor) > 0}
						<p>
							Valor total extensión: <span class="font-semibold text-primary"
								>{formatCOP((parseFloat(extension.valor) * extension.cantidad).toString())}</span
							>
						</p>
					{/if}
				</div>
			</div>

			{#if historialExtensiones.length > 0}
				<div class="mt-4">
					<p class="text-sm font-semibold text-text-primary mb-2">Historial de extensiones:</p>
					<div class="space-y-2">
						{#each historialExtensiones as ext}
							<div class="p-2 rounded-lg bg-alt-row/40 border border-border text-sm">
								<div class="flex justify-between items-center">
									<span class="font-semibold text-text-primary">
										{ext.tipo === 'horas' ? `+${ext.cantidad}h` : `+${ext.cantidad}d`}
									</span>
									<span class="font-semibold text-primary">{formatCOP(ext.valorTotal)}</span>
								</div>
								<div class="text-xs text-text-secondary mt-1">
									{ext.usuario ?? 'sistema'} · {ext.createdAt
										? formatDate(ext.createdAt.split(' ')[0])
										: '—'}
									{#if ext.observaciones}
										<span class="ml-2">· {ext.observaciones}</span>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
			{#if cargandoHistorial}
				<p class="text-xs text-text-secondary mt-2">Cargando historial...</p>
			{/if}
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onClose} disabled={extenderando}>Cancelar</button>
		<button class="btn-primary" onclick={onConfirmar} disabled={extenderando}>
			{#if extenderando}
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
				Extendiendo...
			{:else}
				Aplicar extensión
			{/if}
		</button>
	{/snippet}
</Modal>
