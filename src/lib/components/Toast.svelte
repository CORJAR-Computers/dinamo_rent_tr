<script lang="ts">
	import { onMount } from 'svelte';
	import { fly } from 'svelte/transition';
	import { toasts, dismiss, type ToastType } from '$lib/stores/toast.svelte';

	// Los toasts se insertan siempre tras el mount, así que basta con leer la
	// preferencia una vez (respeta prefers-reduced-motion).
	let reduced = $state(false);
	onMount(() => {
		reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	});

	const styles: Record<ToastType, { border: string; icon: string; iconPath: string }> = {
		success: {
			border: 'border-exito/30',
			icon: 'bg-exito/15 text-exito',
			iconPath:
				'M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
		},
		error: {
			border: 'border-peligro/30',
			icon: 'bg-peligro/15 text-peligro',
			iconPath:
				'M9.75 9.75l4.5 4.5m0-4.5l-4.5 4.5M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
		},
		warning: {
			border: 'border-alerta/30',
			icon: 'bg-alerta/15 text-alerta',
			iconPath:
				'M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z'
		},
		info: {
			border: 'border-primary/30',
			icon: 'bg-primary/15 text-primary',
			iconPath:
				'M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z'
		}
	};
</script>

<div class="fixed bottom-5 right-5 z-[100] flex flex-col gap-2 w-[380px] max-w-[calc(100vw-2rem)]" aria-live="polite">
	{#each toasts as t (t.id)}
		<div
			in:fly={reduced ? { y: 0, duration: 0 } : { y: 16, duration: 200 }}
			class="pointer-events-auto flex items-start gap-3 rounded-xl border bg-surface shadow-lg px-4 py-3 {styles[t.type].border}"
			role="status"
		>
			<span class="w-7 h-7 rounded-full flex items-center justify-center shrink-0 {styles[t.type].icon}">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
					<path stroke-linecap="round" stroke-linejoin="round" d={styles[t.type].iconPath} />
				</svg>
			</span>
			<p class="flex-1 text-sm text-text-primary leading-snug pt-0.5">{t.message}</p>
			<button
				onclick={() => dismiss(t.id)}
				class="text-text-secondary hover:text-text-primary transition-colors p-0.5 rounded"
				aria-label="Cerrar notificación"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
			</button>
		</div>
	{/each}
</div>
