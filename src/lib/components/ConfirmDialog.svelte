<script lang="ts">
	import Modal from './Modal.svelte';

	interface Props {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		cancelLabel?: string;
		loading?: boolean;
		onConfirm: () => void;
		onCancel: () => void;
	}

	let {
		open,
		title,
		message,
		confirmLabel = 'Eliminar',
		cancelLabel = 'Cancelar',
		loading = false,
		onConfirm,
		onCancel
	}: Props = $props();
</script>

<Modal {open} {title} onClose={loading ? () => {} : onCancel} width="max-w-md" dismissible={!loading}>
	{#snippet children()}
		<div class="flex items-start gap-4">
			<div class="w-11 h-11 rounded-xl bg-peligro/10 text-peligro flex items-center justify-center shrink-0">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" /></svg>
			</div>
			<p class="text-sm text-text-primary leading-relaxed pt-1">{message}</p>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onCancel} disabled={loading}>{cancelLabel}</button>
		<button class="btn-danger" onclick={onConfirm} disabled={loading}>
			{#if loading}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Procesando...
			{:else}
				{confirmLabel}
			{/if}
		</button>
	{/snippet}
</Modal>
