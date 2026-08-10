<script lang="ts">
	import { onMount } from 'svelte';
	import {
		mantenimientoApi,
		autoApi,
		businessApi,
		ApiError,
		type Mantenimiento,
		type MantenimientoDatos,
		type TotalesMantenimiento,
		type AlertaKm,
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
	import Icon from '$lib/components/Icon.svelte';

	const sid = () => session.token ?? '';

	let registros = $state<Mantenimiento[]>([]);
	let totales = $state<TotalesMantenimiento | null>(null);
	let alertas = $state<AlertaKm[]>([]);
	let autos = $state<Auto[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);

	// Filtros
	let busqueda = $state('');
	let placaFiltro = $state('');
	let tipoFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<MantenimientoDatos>(defaultForm());
	let formError = $state('');

	// Eliminar
	let eliminarId = $state<number | null>(null);
	let eliminarTipo = $state('');
	let eliminando = $state(false);

	const tiposMantenimiento = $derived(
		lists?.tiposMantenimiento?.length
			? lists.tiposMantenimiento
			: ['Cambio Aceite', 'Frenos', 'Llantas', 'Batería', 'Tecno-Mecánica', 'Lavado General', 'Reparación Mecánica', 'Otro']
	);

	function defaultForm(): MantenimientoDatos {
		return {
			placa: '',
			tipo: '',
			fecha: new Date().toISOString().slice(0, 10),
			descripcion: '',
			observaciones: '',
			costo: '',
			kmProximoCambioAceite: null
		};
	}

	async function cargarTotales() {
		if (!haySesion()) return;
		try {
			totales = await mantenimientoApi.totales(sid());
		} catch {
			/* opcional */
		}
	}

	async function cargarAlertas() {
		if (!haySesion()) return;
		try {
			alertas = await mantenimientoApi.alertasKm(sid());
		} catch {
			alertas = [];
		}
	}

	async function cargar() {
		loading = true;
		try {
			registros = await mantenimientoApi.listar(sid(), busqueda.trim() || undefined, placaFiltro || undefined, tipoFiltro || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los mantenimientos.');
		} finally {
			loading = false;
		}
	}

	async function cargarTodo() {
		if (!haySesion()) return;
		await Promise.all([cargar(), cargarTotales(), cargarAlertas()]);
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
	});

	// Carga inicial + filtros (debounce solo al escribir)
	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const _plac = placaFiltro;
		const _tip = tipoFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			cargarTodo();
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	// Al cambiar de placa en el formulario, autocompletar el km próximo de aceite
	// si el vehículo tiene uno programado; si no, limpiar el campo (evita dejar
	// un km ajeno al vehículo recién seleccionado).
	function alCambiarPlacaForm() {
		const auto = autos.find((a) => a.placa === form.placa);
		form.kmProximoCambioAceite = auto && auto.proximoAceite && auto.proximoAceite > 0 ? auto.proximoAceite : null;
	}

	function abrirNuevo() {
		form = defaultForm();
		editando = false;
		editandoId = null;
		formError = '';
		modalOpen = true;
	}

	function abrirEditar(m: Mantenimiento) {
		form = {
			placa: m.placa,
			tipo: m.tipo,
			fecha: m.fecha,
			descripcion: m.descripcion ?? '',
			observaciones: m.observaciones ?? '',
			costo: m.costo,
			kmProximoCambioAceite: m.kmProximoCambioAceite
		};
		editando = true;
		editandoId = m.id;
		formError = '';
		modalOpen = true;
	}

	async function guardar() {
		formError = '';
		if (!form.placa.trim()) {
			formError = 'La placa es obligatoria.';
			return;
		}
		if (!form.tipo.trim()) {
			formError = 'El tipo de mantenimiento es obligatorio.';
			return;
		}
		if (!form.fecha) {
			formError = 'La fecha es obligatoria.';
			return;
		}
		const costo = parseFloat(form.costo.replace(',', '.'));
		if (Number.isNaN(costo) || costo <= 0) {
			formError = 'El costo debe ser un número mayor que cero.';
			return;
		}
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				await mantenimientoApi.actualizar(sid(), editandoId, form);
				toast.success(`Mantenimiento #${editandoId} actualizado.`);
			} else {
				await mantenimientoApi.crear(sid(), form);
				toast.success('Mantenimiento registrado.');
			}
			modalOpen = false;
			await cargarTodo();
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el mantenimiento.';
		} finally {
			guardando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await mantenimientoApi.eliminar(sid(), eliminarId);
			toast.success('Mantenimiento eliminado.');
			eliminarId = null;
			await cargarTodo();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el mantenimiento.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	// Filas para la tabla (cast a Record para DataTable genérico)
	const tablaRegistros = $derived(registros as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'fecha', header: 'Fecha' },
		{ key: 'placa', header: 'Vehículo' },
		{ key: 'tipo', header: 'Tipo' },
		{ key: 'descripcion', header: 'Descripción' },
		{ key: 'costo', header: 'Costo', align: 'right' as const },
		{ key: 'kmAceite', header: 'Próx. aceite' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];

	// Texto de la alerta por km (aceite/frenos)
	function textoAlerta(a: AlertaKm): string {
		if (a.critica) {
			return `${a.tipo}: vencido · km ${a.kmActual.toLocaleString('es-CO')} > ${a.kmProximo.toLocaleString('es-CO')} km`;
		}
		return `${a.tipo} en ${a.kmRestante.toLocaleString('es-CO')} km (próx. ${a.kmProximo.toLocaleString('es-CO')} km)`;
	}
</script>

<svelte:head>
	<title>Mantenimiento — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Mantenimiento</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{registros.length} registro{registros.length === 1 ? '' : 's'} de mantenimiento
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Registrar Mantenimiento
		</button>
	</div>

	<!-- Alertas por kilometraje -->
	{#if alertas.length > 0}
		<div class="rounded-xl border {alertas.some((a) => a.critica) ? 'border-peligro/30 bg-peligro/5' : 'border-alerta/25 bg-alerta/5'} px-4 py-3">
			<div class="flex items-start gap-2">
				<span class="shrink-0 {alertas.some((a) => a.critica) ? 'text-peligro' : 'text-alerta'}">
					<Icon name="alert" class="w-5 h-5" />
				</span>
				<div class="grow">
					<p class="text-sm font-semibold text-text-primary">Alertas por kilometraje</p>
					<ul class="mt-1 space-y-0.5">
						{#each alertas as a}
							<li class="text-xs {a.critica ? 'text-peligro font-medium' : 'text-text-secondary'}">
								<span class="font-mono font-semibold">{a.placa}</span> · {textoAlerta(a)}
							</li>
						{/each}
					</ul>
				</div>
			</div>
		</div>
	{/if}

	<!-- Resumen de totales -->
	<div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
		<div class="card p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Total invertido</p>
			<p class="text-2xl font-bold text-text-primary tabular-nums mt-1">{formatCOP(totales?.totalGeneral, true)}</p>
			<p class="text-[11px] text-text-secondary mt-0.5">{registros.length} mantenimiento{registros.length === 1 ? '' : 's'}</p>
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
					<p class="text-xs text-text-secondary">Sin mantenimientos por placa</p>
				{/if}
			</div>
		</div>
		<div class="card p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Por tipo</p>
			<div class="mt-1 space-y-1">
				{#if totales && totales.porTipo.length > 0}
					{#each totales.porTipo.slice(0, 3) as t}
						<div class="flex items-center justify-between text-sm">
							<span class="text-xs text-text-secondary font-medium">{t.clave}</span>
							<span class="tabular-nums text-text-primary">{formatCOP(t.total, true)}</span>
						</div>
					{/each}
				{:else}
					<p class="text-xs text-text-secondary">Sin mantenimientos por tipo</p>
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
				placeholder="Buscar por placa, tipo o descripción..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={placaFiltro} aria-label="Filtrar por placa">
			<option value="">Todas las placas</option>
			{#each autos as a}
				<option value={a.placa}>{a.placa}</option>
			{/each}
		</select>
		<select class="input w-auto" bind:value={tipoFiltro} aria-label="Filtrar por tipo">
			<option value="">Todos los tipos</option>
			{#each tiposMantenimiento as t}
				<option value={t}>{t}</option>
			{/each}
		</select>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando mantenimientos...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaRegistros}
			emptyTitle="No hay mantenimientos"
			emptyDescription="Registra el primer mantenimiento con el botón «Registrar Mantenimiento»."
			emptyIcon="wrench"
		>
			{#snippet children(col, item)}
				{@const m = item as unknown as Mantenimiento}
				{#if col.key === 'fecha'}
					<span class="whitespace-nowrap text-text-secondary text-xs">{formatDate(m.fecha)}</span>
				{:else if col.key === 'placa'}
					<div class="max-w-[200px]">
						<p class="font-mono text-xs font-semibold text-primary">{m.placa}</p>
						{#if m.vehiculo}
							<p class="text-[11px] text-text-secondary/70 truncate">{m.vehiculo}</p>
						{/if}
					</div>
				{:else if col.key === 'tipo'}
					<span class="inline-flex items-center rounded-full bg-primary/10 px-2.5 py-0.5 text-[11px] font-semibold text-primary whitespace-nowrap">
						{m.tipo}
					</span>
				{:else if col.key === 'descripcion'}
					<div class="max-w-[240px]">
						<p class="text-text-primary truncate">{m.descripcion || '—'}</p>
						{#if m.observaciones}
							<p class="text-[11px] text-text-secondary/70 truncate">{m.observaciones}</p>
						{/if}
					</div>
				{:else if col.key === 'costo'}
					<span class="font-semibold tabular-nums text-text-primary whitespace-nowrap">{formatCOP(m.costo, true)}</span>
				{:else if col.key === 'kmAceite'}
					{#if m.kmProximoCambioAceite && m.kmProximoCambioAceite > 0}
						<span class="text-xs text-text-secondary tabular-nums whitespace-nowrap">{m.kmProximoCambioAceite.toLocaleString('es-CO')} km</span>
					{:else}
						<span class="text-xs text-text-secondary/60">—</span>
					{/if}
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(m)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
							title="Eliminar"
							onclick={() => {
								eliminarId = m.id;
								eliminarTipo = m.tipo;
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
	title={editando ? `Editar mantenimiento #${editandoId}` : 'Registrar mantenimiento'}
	subtitle="Historial de mantenimiento por vehículo — los costos se registran en COP."
	onClose={() => (modalOpen = false)}
	width="max-w-lg"
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Vehículo" required>
				<select class="input" bind:value={form.placa} onchange={alCambiarPlacaForm}>
					<option value="">Selecciona...</option>
					{#each autos as a}
						<option value={a.placa}>{a.placa} · {a.marca} {a.modelo}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Tipo de mantenimiento" required>
				<select class="input" bind:value={form.tipo}>
					<option value="">Selecciona...</option>
					{#each tiposMantenimiento as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Fecha" required>
				<input class="input" type="date" bind:value={form.fecha} />
			</FormField>
			<FormField label="Costo (COP)" required>
				<input class="input tabular-nums" inputmode="decimal" placeholder="Ej: 350000" bind:value={form.costo} />
			</FormField>
			<div class="col-span-full">
				<FormField label="Descripción">
					<input class="input" placeholder="Ej: Cambio de aceite 15W-40 y filtro" bind:value={form.descripcion} maxlength="250" />
				</FormField>
			</div>
			<div class="col-span-full">
				<FormField label="Km próximo cambio de aceite" hint="Se sincroniza con el vehículo para las alertas por kilometraje.">
					<input
						class="input tabular-nums"
						type="number"
						min="0"
						step="1"
						placeholder="Ej: 50000"
						bind:value={form.kmProximoCambioAceite}
					/>
				</FormField>
			</div>
			<div class="col-span-full">
				<FormField label="Observaciones">
					<textarea class="input min-h-[80px] resize-y" placeholder="Detalles adicionales (opcional)" bind:value={form.observaciones} maxlength="2000"></textarea>
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
				{editando ? 'Guardar cambios' : 'Registrar mantenimiento'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar mantenimiento"
	message={`¿Seguro que deseas eliminar el mantenimiento de tipo «${eliminarTipo}»? Esta acción no se puede deshacer.`}
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>
