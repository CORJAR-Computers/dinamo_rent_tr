<script lang="ts">
	import Icon from './Icon.svelte';

	interface Props {
		title: string;
		description?: string;
		/** Nombre del icono del set Heroicons (ver Icon.svelte) */
		icon?: string;
		actionLabel?: string;
		onAction?: () => void;
	}

	let { title, description, icon = 'search', actionLabel, onAction }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center text-center py-14 px-6">
	<div class="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center mb-4 text-primary">
		<Icon name={icon} class="w-8 h-8" />
	</div>
	<h3 class="text-base font-semibold text-text-primary">{title}</h3>
	{#if description}
		<p class="text-sm text-text-secondary mt-1 max-w-sm">{description}</p>
	{/if}
	{#if actionLabel}
		<button class="btn-outline mt-5" onclick={onAction}>{actionLabel}</button>
	{/if}
</div>
