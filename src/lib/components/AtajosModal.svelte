<script module lang="ts">
	/** Atajos de teclado de la app (fuente única para el modal y los tests). */
	export const ATAJOS_APP: { teclas: string[]; descripcion: string }[] = [
		{ teclas: ['F1', 'Ctrl+/'], descripcion: 'Abrir o cerrar esta ayuda de atajos' },
		{ teclas: ['Ctrl+K'], descripcion: 'Abrir la búsqueda general de páginas (paleta de comandos)' },
		{ teclas: ['Ctrl+Shift+C'], descripcion: 'Copiar datos de un cliente o vehículo existente (en los modales de crear)' },
		{ teclas: ['Esc'], descripcion: 'Cerrar el modal o diálogo activo' }
	];

	/** ¿El evento corresponde a «abrir ayuda» (F1 o Ctrl+/)? */
	export function esAtajoAyuda(e: KeyboardEvent): boolean {
		if (e.key === 'F1' && !e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) return true;
		if (e.key === '/' && e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) return true;
		return false;
	}
</script>

<script lang="ts">
	import Modal from './Modal.svelte';
	import Icon from './Icon.svelte';

	interface Props {
		open: boolean;
		onClose: () => void;
	}

	let { open, onClose }: Props = $props();
</script>

<Modal
	open={open}
	title="Atajos de teclado"
	subtitle="Atajos disponibles en toda la aplicación"
	onClose={onClose}
	width="max-w-md"
>
	{#snippet children()}
		<ul class="divide-y divide-border/60">
			{#each ATAJOS_APP as atajo}
				<li class="flex items-center justify-between gap-4 py-3">
					<span class="text-sm text-text-primary">{atajo.descripcion}</span>
					<span class="flex items-center gap-1.5 shrink-0">
						{#each atajo.teclas as tecla, i}
							{#if i > 0}<span class="text-xs text-text-secondary">o</span>{/if}
							<kbd class="inline-flex items-center rounded-md border border-border bg-alt-row/60 px-2 py-1 text-[11px] font-mono text-text-primary leading-none">{tecla}</kbd>
						{/each}
					</span>
				</li>
			{/each}
		</ul>

		<p class="mt-4 pt-3 border-t border-border text-xs text-text-secondary flex items-start gap-1.5">
			<Icon name="lightbulb" class="w-3.5 h-3.5 shrink-0 mt-0.5 text-alerta" />
			<span>
				Pulsa <kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">F1</kbd>
				o <kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">Ctrl+/</kbd>
				en cualquier momento para volver a ver esta lista.
			</span>
		</p>
	{/snippet}
</Modal>
