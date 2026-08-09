<script lang="ts">
	import Icon from './Icon.svelte';

	interface Props {
		title: string;
		description: string;
		icon?: string;
		badge?: string;
	}

	let { title, description, icon = 'car', badge = 'Fase 6' }: Props = $props();
</script>

<div class="card p-10 flex flex-col items-center justify-center text-center min-h-[60vh]">
	<div class="w-20 h-20 rounded-2xl bg-primary/10 flex items-center justify-center mb-6 text-primary">
		<Icon name={icon} class="w-10 h-10" />
	</div>
	<h2 class="text-2xl font-bold text-text-primary mb-2">{title}</h2>
	<p class="text-text-secondary max-w-md mb-6">{description}</p>
	<span class="text-xs font-semibold px-3 py-1.5 rounded-full bg-primary/10 text-primary uppercase tracking-wide">
		{badge}
	</span>
</div>
