<script lang="ts">
	import EmptyState from './EmptyState.svelte';

	interface Column {
		key: string;
		header: string;
		class?: string;
		align?: 'left' | 'center' | 'right';
	}

	interface Props {
		columns: Column[];
		items: Record<string, unknown>[];
		/** Snippet opcional: children(column, item) para celdas personalizadas */
		children?: import('svelte').Snippet<[Column, Record<string, unknown>]>;
		emptyTitle?: string;
		emptyDescription?: string;
		emptyIcon?: string;
	}

	let {
		columns,
		items,
		children,
		emptyTitle = 'Sin registros',
		emptyDescription,
		emptyIcon = 'document'
	}: Props = $props();

	const alignClass = (align?: string) =>
		align === 'right' ? 'text-right' : align === 'center' ? 'text-center' : 'text-left';
</script>

<div class="card overflow-hidden">
	<div class="overflow-x-auto">
		<table class="w-full text-sm">
			<thead>
				<tr class="bg-alt-row/70 border-b border-border">
					{#each columns as col}
						<th
							class="px-4 py-3 text-[11px] font-bold uppercase tracking-wider text-text-secondary whitespace-nowrap {alignClass(col.align)} {col.class ?? ''}"
						>
							{col.header}
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#if items.length === 0}
					<tr>
						<td colspan={columns.length} class="px-4">
							<EmptyState title={emptyTitle} description={emptyDescription} icon={emptyIcon} />
						</td>
					</tr>
				{:else}
					{#each items as item}
						<tr class="border-b border-border/60 last:border-0 hover:bg-alt-row/50 transition-colors">
							{#each columns as col}
								<td class="px-4 py-3 align-middle text-text-primary {alignClass(col.align)} {col.class ?? ''}">
									{#if children}
										{@render children(col, item)}
									{:else}
										<span class="block truncate max-w-[280px]">{String(item[col.key] ?? '—')}</span>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</div>
