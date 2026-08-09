<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		open: boolean;
		title: string;
		subtitle?: string;
		onClose: () => void;
		width?: string;
		/** Evita cerrar con click fuera / Esc */
		dismissible?: boolean;
		children?: import('svelte').Snippet;
		footer?: import('svelte').Snippet;
	}

	let { open, title, subtitle, onClose, width = 'max-w-xl', dismissible = true, children, footer }: Props = $props();

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && dismissible) onClose();
	}

	$effect(() => {
		if (open && typeof document !== 'undefined') {
			document.addEventListener('keydown', handleKey);
			// Bloquear scroll del fondo
			const prev = document.body.style.overflow;
			document.body.style.overflow = 'hidden';
			return () => {
				document.removeEventListener('keydown', handleKey);
				document.body.style.overflow = prev;
			};
		}
	});
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto overscroll-contain p-4 sm:p-6"
		role="dialog"
		aria-modal="true"
		aria-label={title}
	>
		<button
			class="fixed inset-0 bg-slate-900/50 backdrop-blur-[2px] cursor-default animate-[modal-fade-in_150ms_ease-out]"
			onclick={() => (dismissible ? onClose() : null)}
			aria-label="Cerrar diálogo"
			tabindex="-1"
		></button>

		<div
			class="relative w-full {width} mt-4 sm:mt-10 card shadow-2xl flex flex-col max-h-[calc(100vh-4rem)] animate-[modal-pop-in_180ms_ease-out]"
		>
			<!-- Header -->
			<div class="flex items-start justify-between gap-4 px-6 py-4 border-b border-border shrink-0">
				<div>
					<h2 class="text-lg font-bold text-text-primary leading-tight">{title}</h2>
					{#if subtitle}
						<p class="text-xs text-text-secondary mt-0.5">{subtitle}</p>
					{/if}
				</div>
				<button
					onclick={onClose}
					class="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-alt-row transition-colors shrink-0"
					aria-label="Cerrar"
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
				</button>
			</div>

			<!-- Cuerpo -->
			<div class="px-6 py-5 overflow-y-auto grow">
				{@render children?.()}
			</div>

			<!-- Footer -->
			{#if footer}
				<div class="px-6 py-4 border-t border-border flex items-center justify-end gap-2 shrink-0 bg-surface/50 rounded-b-xl">
					{@render footer()}
				</div>
			{/if}
		</div>
	</div>
{/if}
