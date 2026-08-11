<script module lang="ts">
	/** Normaliza texto para búsqueda sin acentos ni mayúsculas. */
	export function normalizarTexto(s: string): string {
		return s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');
	}

	/** ¿El evento corresponde a «abrir la paleta de comandos» (Ctrl+K / Cmd+K)? */
	export function esAtajoPaleta(e: KeyboardEvent): boolean {
		if (e.key.toLowerCase() !== 'k') return false;
		if (!e.ctrlKey && !e.metaKey) return false; // Ctrl (Windows/Linux) o Cmd (macOS)
		if (e.shiftKey || e.altKey) return false;
		return true;
	}

	export interface ItemMenu {
		label: string;
		href: string;
		icon: string;
		adminOnly?: boolean;
		/** Roles con acceso (espejo de roles_con_informes para el menú). */
		roles?: string[];
	}
	export interface SeccionMenu {
		section: string;
		items: ItemMenu[];
	}
</script>

<script lang="ts">
	import { goto } from '$app/navigation';
	import Modal from './Modal.svelte';
	import Icon from './Icon.svelte';

	interface ResultadoPaleta {
		label: string;
		href: string;
		icon: string;
		seccion: string;
		actual: boolean;
	}

	interface Props {
		open: boolean;
		onClose: () => void;
		menu: SeccionMenu[];
		rol?: string | null;
		/** Ruta actual (sin parámetros) para marcar la página en la que estás. */
		actualPath?: string;
	}

	let { open, onClose, menu, rol = null, actualPath = '' }: Props = $props();

	let termino = $state('');
	let indice = $state(0);
	let inputEl: HTMLInputElement | undefined = $state();

	// Nombres de icono del menú (paridad con el sidebar) → set Heroicons del <Icon>
	function iconoPara(nombre: string): string {
		const map: Record<string, string> = {
			dashboard: 'chart',
			calendar: 'calendar',
			rentas: 'clipboard',
			reservas: 'document',
			clientes: 'users',
			autos: 'car',
			mantenimiento: 'wrench',
			usuarios: 'users',
			auditoria: 'document',
			comparendos: 'check',
			alertas: 'alert',
			informes: 'chart',
			gastos: 'money'
		};
		return map[nombre] ?? 'chart';
	}

	const resultados = $derived.by(() => {
		const q = normalizarTexto(termino);
		const out: ResultadoPaleta[] = [];
		for (const grupo of menu) {
			for (const item of grupo.items) {
				if (item.adminOnly && rol !== 'Administrador') continue;
				if (item.roles && !item.roles.includes(rol ?? '')) continue;
				if (!q) {
					out.push({
						label: item.label,
						href: item.href,
						icon: iconoPara(item.icon),
						seccion: grupo.section,
						actual: actualPath === item.href
					});
					continue;
				}
				const etiqueta = normalizarTexto(item.label);
				const seccion = normalizarTexto(grupo.section);
				if (etiqueta.includes(q) || seccion.includes(q)) {
					out.push({
						label: item.label,
						href: item.href,
						icon: iconoPara(item.icon),
						seccion: grupo.section,
						actual: actualPath === item.href
					});
				}
			}
		}
		return out;
	});

	// Al abrir: limpiar búsqueda, reiniciar selección y enfocar el campo
	$effect(() => {
		if (open) {
			termino = '';
			indice = 0;
			requestAnimationFrame(() => inputEl?.focus());
		}
	});

	// Mantener la selección dentro de los resultados al filtrar
	$effect(() => {
		if (indice > resultados.length - 1) indice = Math.max(0, resultados.length - 1);
	});

	function navegar(r: ResultadoPaleta) {
		goto(r.href);
		onClose();
	}

	function onInputKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			indice = Math.min(indice + 1, resultados.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			indice = Math.max(indice - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const r = resultados[indice];
			if (r) navegar(r);
		}
		// Esc lo cierra el Modal (listener global de documento)
	}
</script>

<Modal
	open={open}
	title="Ir a…"
	subtitle="Busca una página por nombre o sección"
	onClose={onClose}
	width="max-w-lg"
>
	{#snippet children()}
		<div class="relative">
			<Icon
				name="search"
				class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary pointer-events-none"
			/>
			<input
				bind:this={inputEl}
				bind:value={termino}
				oninput={() => (indice = 0)}
				onkeydown={onInputKeydown}
				class="input pl-9 pr-3"
				placeholder="Buscar página… (ej: rentas, admin, finanzas)"
				aria-label="Buscar página"
				role="combobox"
				aria-expanded="true"
				aria-controls="paleta-resultados"
				aria-activedescendant={resultados[indice] ? `paleta-op-${indice}` : undefined}
				autocomplete="off"
			/>
		</div>

		{#if resultados.length === 0}
			<p class="text-sm text-text-secondary text-center py-6">
				Sin resultados para «{termino.trim()}».
			</p>
		{:else}
			<ul id="paleta-resultados" class="mt-3 max-h-72 overflow-y-auto space-y-0.5" role="listbox" aria-label="Resultados de búsqueda">
				{#each resultados as r, i}
					<li role="none">
						<button
							type="button"
							id={`paleta-op-${i}`}
							role="option"
							aria-selected={i === indice}
							onmouseenter={() => (indice = i)}
							onclick={() => navegar(r)}
							class={'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left text-sm transition-colors ' +
								(i === indice ? 'bg-primary/10 text-primary ' : 'text-text-primary hover:bg-alt-row ')}
						>
							<span class={'shrink-0 ' + (i === indice ? 'text-primary' : 'text-text-secondary')}>
								<Icon name={r.icon} class="w-4 h-4" />
							</span>
							<span class="flex-1 min-w-0 truncate">{r.label}</span>
							<span class="text-[10px] uppercase tracking-wider text-text-secondary shrink-0">{r.seccion}</span>
							{#if r.actual}
								<span class="text-[10px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded bg-primary/10 text-primary shrink-0">
									Actual
								</span>
							{/if}
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		<p class="mt-3 pt-3 border-t border-border text-xs text-text-secondary flex items-center gap-x-4 gap-y-1.5 flex-wrap">
			<span class="flex items-center gap-1.5">
				<kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">↑</kbd>
				<kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">↓</kbd>
				navegar
			</span>
			<span class="flex items-center gap-1.5">
				<kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">Enter</kbd>
				abrir
			</span>
			<span class="flex items-center gap-1.5">
				<kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">Esc</kbd>
				cerrar
			</span>
			<span class="ml-auto flex items-center gap-1.5">
				<kbd class="inline-flex items-center rounded border border-border bg-alt-row/60 px-1 py-0.5 text-[10px] font-mono leading-none">Ctrl+K</kbd>
				en cualquier momento
			</span>
		</p>
	{/snippet}
</Modal>
