<script lang="ts">
	import { onMount } from 'svelte';
	import { auditoriaApi, ApiError, type AuditoriaEvento } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDateTime } from '$lib/utils/format';
	import { guardRole, guardSesion, haySesion, tieneRol } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';

	const sid = () => session.token ?? '';
	const POR_PAGINA = 50;

	let eventos = $state<AuditoriaEvento[]>([]);
	let total = $state(0);
	let pagina = $state(1);
	let loading = $state(true);

	// Filtros
	let busqueda = $state('');
	let usuarioFiltro = $state('');
	let accionFiltro = $state('');
	let fechaDesde = $state('');
	let fechaHasta = $state('');
	let usuarios = $state<string[]>([]);
	let acciones = $state<string[]>([]);
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	async function cargar(paginaDestino = 1) {
		// Guard de sesión + rol: nunca consultar sin sesión ni si el usuario
		// no es administrador (cubre también el debounce durante una redirección).
		if (!haySesion()) return;
		if (!tieneRol(['Administrador'])) return;
		pagina = paginaDestino;
		loading = true;
		try {
			const r = await auditoriaApi.listar(
				sid(),
				{
					usuario: usuarioFiltro,
					accion: accionFiltro,
					fechaDesde,
					fechaHasta,
					busqueda: busqueda.trim() || undefined
				},
				paginaDestino,
				POR_PAGINA
			);
			eventos = r.eventos;
			total = r.total;
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo consultar la auditoría.');
		} finally {
			loading = false;
		}
	}

	// Cargar opciones de filtros y la primera página al montar
	onMount(async () => {
		// Guard de sesión + rol: solo administradores ven la auditoría.
		// El menú ya oculta la ruta, pero esto protege el acceso directo por URL.
		if (!guardSesion()) return;
		if (!guardRole(['Administrador'], '/dashboard')) return;
		try {
			const [u, a] = await Promise.all([
				auditoriaApi.usuarios(sid()).catch(() => []),
				auditoriaApi.acciones(sid()).catch(() => [])
			]);
			usuarios = u;
			acciones = a;
		} catch {
			/* opcional */
		}
		cargar(1);
	});

	// Filtros → recarga (debounce solo para la búsqueda libre). El onMount ya
	// dispara la carga inicial, así que este efecto solo actúa ante cambios.
	let primerEfecto = true;
	$effect(() => {
		const term = busqueda;
		const u = usuarioFiltro;
		const a = accionFiltro;
		const d = fechaDesde;
		const h = fechaHasta;
		if (primerEfecto) {
			primerEfecto = false;
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(1), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	function limpiarFiltros() {
		usuarioFiltro = '';
		accionFiltro = '';
		fechaDesde = '';
		fechaHasta = '';
		busqueda = '';
		cargar(1);
	}

	const totalPaginas = $derived(Math.max(1, Math.ceil(total / POR_PAGINA)));

	function irPagina(p: number) {
		if (p < 1 || p > totalPaginas) return;
		cargar(p);
	}

	// Filas para la tabla (cast a Record para DataTable genérico)
	const tablaEventos = $derived(eventos as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'fecha', header: 'Fecha' },
		{ key: 'usuario', header: 'Usuario' },
		{ key: 'accion', header: 'Acción' },
		{ key: 'mensaje', header: 'Detalle' },
		{ key: 'ip', header: 'IP' }
	];

	// Colores por tipo de acción (badge)
	function badgeClass(accion: string): string {
		const a = accion.toUpperCase();
		if (a.includes('LOGIN OK')) return 'bg-exito/10 text-exito';
		if (a.includes('LOGIN FALLIDO') || a.includes('ACCESO DENEGADO') || a.includes('BLOQUEADO')) return 'bg-peligro/10 text-peligro';
		if (a.includes('ELIMINADO')) return 'bg-peligro/10 text-peligro';
		if (a.includes('CREADO')) return 'bg-exito/10 text-exito';
		if (a.includes('CONTRASEÑA') || a.includes('PASSWORD')) return 'bg-alerta/10 text-alerta';
		return 'bg-primary/10 text-primary';
	}
</script>

<svelte:head>
	<title>Auditoría — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Auditoría</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{total.toLocaleString('es-CO')} evento{total === 1 ? '' : 's'} registrado{total === 1 ? '' : 's'}
			</p>
		</div>
		<button class="btn-ghost !px-3 !py-1.5 text-xs" onclick={limpiarFiltros}>
			Limpiar filtros
		</button>
	</div>

	<!-- Filtros -->
	<div class="card p-3 flex flex-wrap items-end gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por usuario, acción o detalle..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={usuarioFiltro} aria-label="Filtrar por usuario">
			<option value="">Todos los usuarios</option>
			{#each usuarios as u}
				<option value={u}>{u}</option>
			{/each}
		</select>
		<select class="input w-auto" bind:value={accionFiltro} aria-label="Filtrar por acción">
			<option value="">Todas las acciones</option>
			{#each acciones as a}
				<option value={a}>{a}</option>
			{/each}
		</select>
		<label class="flex flex-col gap-1 text-[11px] font-medium text-text-secondary">
			Desde
			<input class="input" type="date" bind:value={fechaDesde} />
		</label>
		<label class="flex flex-col gap-1 text-[11px] font-medium text-text-secondary">
			Hasta
			<input class="input" type="date" bind:value={fechaHasta} />
		</label>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Consultando auditoría...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaEventos}
			emptyTitle="Sin eventos"
			emptyDescription="No hay eventos de auditoría con los filtros seleccionados."
			emptyIcon="document"
		>
			{#snippet children(col, item)}
				{@const e = item as unknown as AuditoriaEvento}
				{#if col.key === 'fecha'}
					<span class="whitespace-nowrap text-text-secondary text-xs tabular-nums">{formatDateTime(e.fecha)}</span>
				{:else if col.key === 'usuario'}
					<span class="font-mono text-xs font-semibold text-primary">{e.usuario || '—'}</span>
				{:else if col.key === 'accion'}
					<span class="inline-flex items-center rounded-full px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {badgeClass(e.accion)}">
						{e.accion}
					</span>
				{:else if col.key === 'mensaje'}
					<div class="max-w-[380px]">
						<p class="text-text-secondary text-xs font-mono truncate">{e.mensaje || '—'}</p>
					</div>
				{:else if col.key === 'ip'}
					<span class="text-xs text-text-secondary/70 font-mono">{e.ip || '—'}</span>
				{:else}
					<span>{String(item[col.key] ?? '—')}</span>
				{/if}
			{/snippet}
		</DataTable>
	{/if}

	<!-- Paginación -->
	{#if total > POR_PAGINA}
		<div class="flex items-center justify-between gap-3">
			<p class="text-xs text-text-secondary">
				Página {pagina} de {totalPaginas} · {total.toLocaleString('es-CO')} eventos
			</p>
			<div class="flex items-center gap-1.5">
				<button
					class="btn-ghost !px-3 !py-1.5 text-xs"
					disabled={pagina <= 1}
					onclick={() => irPagina(pagina - 1)}
				>
					← Anterior
				</button>
				{#if pagina > 3}
					<button class="btn-ghost !px-2.5 !py-1.5 text-xs" onclick={() => irPagina(1)}>1</button>
					<span class="text-xs text-text-secondary/60">…</span>
				{/if}
				{#each Array.from({ length: Math.min(5, totalPaginas) }, (_, i) => {
					// ventana centrada en la página actual
					const inicio = Math.max(1, Math.min(pagina - 2, totalPaginas - 4));
					return inicio + i;
				}) as p}
					<button
						class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors {p === pagina ? 'bg-primary text-white' : 'btn-ghost'}"
						onclick={() => irPagina(p)}
					>
						{p}
					</button>
				{/each}
				{#if pagina < totalPaginas - 2}
					<span class="text-xs text-text-secondary/60">…</span>
					<button class="btn-ghost !px-2.5 !py-1.5 text-xs" onclick={() => irPagina(totalPaginas)}>
						{totalPaginas}
					</button>
				{/if}
				<button
					class="btn-ghost !px-3 !py-1.5 text-xs"
					disabled={pagina >= totalPaginas}
					onclick={() => irPagina(pagina + 1)}
				>
					Siguiente →
				</button>
			</div>
		</div>
	{/if}
</div>
