<script lang="ts">
	interface Props {
		label: string;
		required?: boolean;
		hint?: string;
		error?: string;
		children?: import('svelte').Snippet;
	}

	let { label, required = false, hint, error, children }: Props = $props();
</script>

<div class="mb-4">
	{#if label}
		<span class="label">
			{label}
			{#if required}<span class="text-peligro"> *</span>{/if}
		</span>
	{/if}
	{@render children?.()}
	{#if error}
		<p class="mt-1 text-xs text-peligro flex items-center gap-1">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" /></svg>
			{error}
		</p>
	{:else if hint}
		<p class="mt-1 text-xs text-text-secondary/70">{hint}</p>
	{/if}
</div>
