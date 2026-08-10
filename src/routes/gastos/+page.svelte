<script lang="ts">
	import { onMount } from 'svelte';
	import {
		gastoApi,
		autoApi,
		businessApi,
		ApiError,
		type Gasto,
		type GastoDatos,
		type TotalesGastos,
		type Auto,
		type BusinessLists
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';

	const sid = () => session.token ?? '';

	let gastos = $state<Gasto[]>([]);
	let totales = $state<TotalesGastos | null>(null);
	let autos = $state<Auto[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);

	// Filtros
	let busqueda = $state('');
	let placaFiltro = $state('');
	let categoriaFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<GastoDatos>(defaultForm());
	let formError = $state('');

	// Eliminar
	let eliminarId = $state<number | null>(null);
	let eliminarDesc = $state('');
	let eliminando = $state(false);

	const categorias = $derived(
		lists?.tiposGasto?.length ? lists.tiposGasto : ['Combustible', 'Peajes', 'Lavado', 'Mantenimiento', 'Repuestos', 'Parqueadero', 'Seguros', 'Multas', 'Papelería', 'Otros']
	);

	function defaultForm(): GastoDatos {
		return {
			placa: '',
			fecha: new Date().toISOString().slice(0, 10),
			categoria: '',
			descripcion: '',
			monto: '',
			comprobante: ''
		};
	}

	async function cargarTotales() {
		if (!haySesion()) return;
		try {
			totales = await gastoApi.totales(sid());
		} catch {
			/* opcional: los totales no bloquean la tabla */
		}
	}

	async function cargar() {
		loading = true;
		try {
			gastos = await gastoApi.listar(sid(), busqueda.trim() || undefined, placaFiltro || undefined, categoriaFiltro || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los gastos.');
		} finally {
			loading = false;
		}
	}

	async function cargarTodo() {
		if (!haySesion()) return;
		await Promise.all([cargar(), cargarTotales()]);
	}

	onMount(async () => {
		if (!guardSesion()) return;
		try {
			const [l, a] = await Promise.all([
				businessApi.listas(sid()).catch(() => null),
				autoApi.listar(sid(), undefined, undefined).catch(() => [])
			]);
			lists = l;
			autos = a;
		} catch {
			/* opcional */
		}
		// La carga inicial de gastos la dispara el $effect de filtros (una sola vez)
	});

	// Carga inicial + filtros (debounce solo al escribir)
	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const _plac = placaFiltro;
		const _cat = categoriaFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			cargarTodo();
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	function abrirNuevo() {
		form = defaultForm();
		editando = false;
		editandoId = null;
		formError = '';
		modalOpen = true;
	}

	function abrirEditar(g: Gasto) {
		form = {
			placa: g.placa ?? '',
			fecha: g.fecha,
			categoria: g.categoria,
			descripcion: g.descripcion,
			monto: g.monto,
			comprobante: g.comprobante ?? ''
		};
		editando = true;
		editandoId = g.id;
		formError = '';
		modalOpen = true;
	}

	async function guardar() {
		formError = '';
		if (!form.fecha) {
			formError = 'La fecha es obligatoria.';
			return;
		}
		if (!form.categoria.trim()) {
			formError = 'La categoría es obligatoria.';
			return;
		}
		if (!form.descripcion.trim()) {
			formError = 'La descripción es obligatoria.';
			return;
		}
		const monto = parseFloat(form.monto.replace(',', '.'));
		if (Number.isNaN(monto) || monto <= 0) {
			formError = 'El monto debe ser un número mayor que cero.';
			return;
		}
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				await gastoApi.actualizar(sid(), editandoId, form);
				toast.success(`Gasto #${editandoId} actualizado.`);
			} else {
				await gastoApi.crear(sid(), form);
				toast.success('Gasto registrado.');
			}
			modalOpen = false;
			await cargarTodo();
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el gasto.';
		} finally {
			guardando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await gastoApi.eliminar(sid(), eliminarId);
			toast.success('Gasto eliminado.');
			eliminarId = null;
			await cargarTodo();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el gasto.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	// Filas para la tabla (cast a Record para DataTable genérico)
	const tablaGastos = $derived(gastos as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'fecha', header: 'Fecha' },
		{ key: 'placa', header: 'Placa' },
		{ key: 'categoria', header: 'Categoría' },
		{ key: 'descripcion', header: 'Descripción' },
		{ key: 'monto', header: 'Monto', align: 'right' as const },
		{ key: 'comprobante', header: 'Comprobante' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];

	// Total del mes actual — lo calcula el backend (SUM sobre la tabla completa,
	// sin verse afectado por los filtros activos de la tabla).
	const totalMes = $derived(totales?.totalMes);
</script>

<svelte:head>
	<title>Gastos — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Caja Menor · Gastos</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{gastos.length} gasto{gastos.length === 1 ? '' : 's'} registrado{gastos.length === 1 ? '' : 's'}
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Registrar Gasto
		</button>
	</div>

	<!-- Resumen de totales -->
	<div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
		<div class="card p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Total general</p>
			<p class="text-2xl font-bold text-text-primary tabular-nums mt-1">{formatCOP(totales?.totalGeneral, true)}</p>
			<p class="text-[11px] text-text-secondary mt-0.5">Este mes: {formatCOP(totalMes, true)}</p>
		</div>
		<div class="card p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Por placa</p>
			<div class="mt-1 space-y-1">
				{#if totales && totales.porPlaca.length > 0}
					{#each totales.porPlaca.slice(0, 3) as t}
						<div class="flex items-center justify-between text-sm">
							<span class="font-mono text-xs text-primary font-semibold">{t.clave}</span>
							<span class="tabular-nums text-text-primary">{formatCOP(t.total, true)}</span>
						</div>
					{/each}
				{:else}
					<p class="text-xs text-text-secondary">Sin gastos por placa</p>
				{/if}
			</div>
		</div>
		<div class="card p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Por categoría</p>
			<div class="mt-1 space-y-1">
				{#if totales && totales.porCategoria.length > 0}
					{#each totales.porCategoria.slice(0, 3) as t}
						<div class="flex items-center justify-between text-sm">
							<span class="text-xs text-text-secondary font-medium">{t.clave}</span>
							<span class="tabular-nums text-text-primary">{formatCOP(t.total, true)}</span>
						</div>
					{/each}
				{:else}
					<p class="text-xs text-text-secondary">Sin gastos por categoría</p>
				{/if}
			</div>
		</div>
	</div>

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por descripción, placa o comprobante..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={placaFiltro} aria-label="Filtrar por placa">
			<option value="">Todas las placas</option>
			{#each autos as a}
				<option value={a.placa}>{a.placa}</option>
			{/each}
		</select>
		<select class="input w-auto" bind:value={categoriaFiltro} aria-label="Filtrar por categoría">
			<option value="">Todas las categorías</option>
			{#each categorias as c}
				<option value={c}>{c}</option>
			{/each}
		</select>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando gastos...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaGastos}
			emptyTitle="No hay gastos"
			emptyDescription="Registra el primer gasto con el botón «Registrar Gasto»."
			emptyIcon="money"
		>
			{#snippet children(col, item)}
				{@const g = item as unknown as Gasto}
				{#if col.key === 'fecha'}
					<span class="whitespace-nowrap text-text-secondary text-xs">{formatDate(g.fecha)}</span>
				{:else if col.key === 'placa'}
					{#if g.placa}
						<span class="font-mono text-xs font-semibold text-primary">{g.placa}</span>
					{:else}
						<span class="text-xs text-text-secondary/60">—</span>
					{/if}
				{:else if col.key === 'categoria'}
					<span class="inline-flex items-center rounded-full bg-primary/10 px-2.5 py-0.5 text-[11px] font-semibold text-primary whitespace-nowrap">
						{g.categoria}
					</span>
				{:else if col.key === 'descripcion'}
					<div class="max-w-[260px]">
						<p class="font-medium text-text-primary truncate">{g.descripcion}</p>
						{#if g.usuario && g.usuario !== 'Sistema'}
							<p class="text-[11px] text-text-secondary/70 truncate">por {g.usuario}</p>
						{/if}
					</div>
				{:else if col.key === 'monto'}
					<span class="font-semibold tabular-nums text-text-primary whitespace-nowrap">{formatCOP(g.monto, true)}</span>
				{:else if col.key === 'comprobante'}
					{#if g.comprobante}
						<span class="text-xs text-text-secondary font-mono">{g.comprobante}</span>
					{:else}
						<span class="text-xs text-text-secondary/60">—</span>
					{/if}
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(g)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
							title="Eliminar"
							onclick={() => {
								eliminarId = g.id;
								eliminarDesc = g.descripcion;
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

<!-- Modal crear/editar -->
<Modal
	open={modalOpen}
	title={editando ? `Editar gasto #${editandoId}` : 'Registrar gasto'}
	subtitle="Caja menor — los montos se registran en pesos colombianos (COP)."
	onClose={() => (modalOpen = false)}
	width="max-w-lg"
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Fecha" required>
				<input class="input" type="date" bind:value={form.fecha} />
			</FormField>
			<FormField label="Placa" hint="Opcional — gasto general de la empresa.">
				<select class="input" bind:value={form.placa}>
					<option value="">Sin vehículo</option>
					{#each autos as a}
						<option value={a.placa}>{a.placa} · {a.marca} {a.modelo}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Categoría" required>
				<select class="input" bind:value={form.categoria}>
					<option value="">Selecciona...</option>
					{#each categorias as c}
						<option value={c}>{c}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Monto (COP)" required>
				<input class="input tabular-nums" inputmode="decimal" placeholder="Ej: 120000" bind:value={form.monto} />
			</FormField>
			<div class="col-span-full">
				<FormField label="Descripción" required>
					<input class="input" placeholder="Ej: Cambio de aceite 15W-40" bind:value={form.descripcion} maxlength="200" />
				</FormField>
			</div>
			<div class="col-span-full">
				<FormField label="Comprobante" hint="Número de factura, recibo o remisión.">
					<input class="input" placeholder="Ej: F-000123" bind:value={form.comprobante} maxlength="50" />
				</FormField>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
		<button class="btn-primary" onclick={guardar} disabled={guardando}>
			{#if guardando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				{editando ? 'Guardar cambios' : 'Registrar gasto'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar gasto"
	message={`¿Seguro que deseas eliminar el gasto «${eliminarDesc}»? Esta acción no se puede deshacer.`}
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>
