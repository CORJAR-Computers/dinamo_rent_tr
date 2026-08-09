<script lang="ts">
	import { onMount } from 'svelte';
	import { clienteApi, businessApi, ApiError, type Cliente, type ClienteConPii, type BusinessLists } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import PiiKeyDialog from '$lib/components/PiiKeyDialog.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import ClienteFormModal from '$lib/components/ClienteFormModal.svelte';

	const sid = () => session.token ?? '';

	let clientes = $state<ClienteConPii[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);
	let piiDialogOpen = $state(false);

	// Filtros
	let busqueda = $state('');
	let estadoFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal crear/editar (formulario reutilizable ClienteFormModal)
	let modalOpen = $state(false);
	let clienteEditando = $state<Cliente | null>(null);

	// Eliminar
	let eliminarId = $state<number | null>(null);
	let eliminarNombre = $state('');
	let eliminando = $state(false);

	// ¿Algún cliente con PII legacy no descifrable?
	const piiLegacy = $derived(clientes.some((c) => c.piiOculto));

	async function cargar() {
		if (!haySesion()) return;
		loading = true;
		try {
			clientes = await clienteApi.listar(sid(), busqueda.trim() || undefined, estadoFiltro || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los clientes.');
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		if (!guardSesion()) return;
		if (!lists) {
			try {
				lists = await businessApi.listas(sid());
			} catch {
				/* opcional */
			}
		}
		// La carga inicial de clientes la dispara el $effect de filtros (una sola vez)
	});

	// Carga inicial + filtros (debounce solo al escribir)
	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const est = estadoFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			cargar();
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	function abrirNuevo() {
		clienteEditando = null;
		modalOpen = true;
	}

	function abrirEditar(c: Cliente) {
		clienteEditando = c;
		modalOpen = true;
	}

	function onClienteGuardado(r: ClienteConPii) {
		modalOpen = false;
		toast.success(`Cliente ${r.cliente.nombreCompleto} ${clienteEditando ? 'actualizado' : 'creado'}.`);
		cargar();
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await clienteApi.eliminar(sid(), eliminarId);
			toast.success(`Cliente ${eliminarNombre} eliminado.`);
			eliminarId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el cliente.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	function licenciaVence(fecha: string | null): string {
		if (!fecha) return '—';
		const d = new Date(fecha + 'T00:00:00');
		const hoy = new Date();
		hoy.setHours(0, 0, 0, 0);
		const dias = Math.round((d.getTime() - hoy.getTime()) / 86_400_000);
		if (dias < 0) return `${formatDate(fecha)} · vencida`;
		if (dias <= 30) return `${formatDate(fecha)} · ${dias}d`;
		return formatDate(fecha);
	}

	// Filas para la tabla (cast a Record para DataTable genérico)
	const tablaClientes = $derived(
		clientes.map((x) => x.cliente) as unknown as Record<string, unknown>[]
	);

	const columnas = [
		{ key: 'documento', header: 'Documento' },
		{ key: 'nombre', header: 'Nombre completo' },
		{ key: 'celular', header: 'Contacto' },
		{ key: 'ciudad', header: 'Ciudad' },
		{ key: 'licencia', header: 'Licencia vence' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Clientes — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Clientes</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{clientes.length} cliente{clientes.length === 1 ? '' : 's'} registrado{clientes.length === 1 ? '' : 's'}
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Nuevo Cliente
		</button>
	</div>

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por nombre, documento o celular..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={estadoFiltro} aria-label="Filtrar por estado">
			<option value="">Todos los estados</option>
			{#each (lists?.estadosCliente ?? ['Activo', 'Inactivo', 'Lista Negra', 'VIP']) as est}
				<option value={est}>{est}</option>
			{/each}
		</select>
	</div>

	{#if piiLegacy}
		<div class="rounded-xl border border-alerta/25 bg-alerta/5 px-4 py-3 text-sm text-alerta flex items-start gap-2">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 mt-0.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
			<span class="grow">
				Algunos clientes antiguos tienen datos de contacto cifrados (Fernet legacy) que requieren la clave original
				<code class="font-mono text-xs bg-surface px-1 py-0.5 rounded">db_encryption_key</code> para mostrarse.
			</span>
			<button class="btn-outline !px-2.5 !py-1 text-[11px] shrink-0" onclick={() => (piiDialogOpen = true)}>
				<span class="inline-flex items-center gap-1.5"><Icon name="lock" class="w-3.5 h-3.5" />Configurar clave</span>
			</button>
		</div>
	{/if}

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando clientes...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaClientes}
			emptyTitle="No hay clientes"
			emptyDescription="Registra el primer cliente con el botón «Nuevo Cliente»."
			emptyIcon="users"
		>
			{#snippet children(col, item)}
				{@const c = item as unknown as Cliente}
				{#if col.key === 'documento'}
					<div>
						<p class="font-bold text-primary tabular-nums">{c.noDoc || '—'}</p>
						<p class="text-[11px] text-text-secondary">{c.tipoDoc || ''}</p>
					</div>
				{:else if col.key === 'nombre'}
					<div class="max-w-[220px]">
						<p class="font-semibold text-text-primary truncate">{c.nombreCompleto}</p>
						{#if c.nacionalidad}
							<p class="text-xs text-text-secondary truncate">{c.nacionalidad}</p>
						{/if}
					</div>
				{:else if col.key === 'celular'}
					<div>
						<p class="text-text-primary tabular-nums">{c.celular || '—'}</p>
						{#if c.email}
							<p class="text-xs text-text-secondary truncate max-w-[180px]">{c.email}</p>
						{/if}
					</div>
				{:else if col.key === 'licencia'}
					<span class="text-text-secondary text-xs whitespace-nowrap">{licenciaVence(c.vencimientoLicencia)}</span>
				{:else if col.key === 'estado'}
					<StatusBadge estado={c.estado} />
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(c)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
							title="Eliminar"
							onclick={() => {
								eliminarId = c.id;
								eliminarNombre = c.nombreCompleto;
							}}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" /></svg>
						</button>
					</div>
				{:else}
					<span>{String(item[col.key] ?? '—')}</span>
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>

<!-- Modal crear/editar (formulario reutilizable) -->
<ClienteFormModal
	open={modalOpen}
	editando={clienteEditando}
	lists={lists}
	clientes={clientes}
	onClose={() => (modalOpen = false)}
	onGuardado={onClienteGuardado}
/>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar cliente"
	message={`¿Seguro que deseas eliminar a ${eliminarNombre}? Esta acción no se puede deshacer.`}
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>

<!-- Diálogo de clave PII (datos legacy Fernet) -->
<PiiKeyDialog
	open={piiDialogOpen}
	onClose={() => (piiDialogOpen = false)}
	onSaved={cargar}
/>
