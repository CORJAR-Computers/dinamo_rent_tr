<script lang="ts" generics="T = Record<string, unknown>">
	import EmptyState from './EmptyState.svelte';

	/**
	 * DataTable genérico (Svelte 5 runes + TypeScript).
	 *
	 * `T` es el tipo de cada fila. Por defecto `Record<string, unknown>`,
	 * lo que lo hace retrocompatible con los consumidores existentes que
	 * todavía hacen `as unknown as Record<string, unknown>[]` (se siguen
	 * compilando sin cambios). Los nuevos consumidores pueden pasar el
	 * tipo concreto (`Renta`, `Cliente`, `Auto`, …) y obtener type-safety
	 * completa en el snippet `children`.
	 *
	 * El `key` de cada columna es un `string` (no `keyof T`) porque los
	 * consumidores lo usan como discriminador virtual dentro del snippet
	 * (`col.key === 'acciones'`), no como acceso directo al campo.
	 */
	interface Column {
		key: string;
		header: string;
		class?: string;
		align?: 'left' | 'center' | 'right';
	}

	interface Props {
		columns: Column[];
		items: T[];
		/** Snippet opcional: children(column, item) para celdas personalizadas */
		children?: import('svelte').Snippet<[Column, T]>;
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

	// Clave de fila estable para el {#each} (R9 del informe): usa `id` si
	// existe, si no el índice del array. Mejora el diffing y evita
	// re-render innecesario al reordenar/filtrar.
	function rowKey(item: T, idx: number): string | number {
		const id = (item as Record<string, unknown>).id;
		return typeof id === 'string' || typeof id === 'number' ? id : `row-${idx}`;
	}
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
					{#each items as item, idx (rowKey(item, idx))}
						<tr class="border-b border-border/60 last:border-0 hover:bg-alt-row/50 transition-colors">
							{#each columns as col}
								<td class="px-4 py-3 align-middle text-text-primary {alignClass(col.align)} {col.class ?? ''}">
									{#if children}
										{@render children(col, item)}
									{:else}
										<span class="block truncate max-w-[280px]">{String((item as Record<string, unknown>)[col.key] ?? '—')}</span>
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
