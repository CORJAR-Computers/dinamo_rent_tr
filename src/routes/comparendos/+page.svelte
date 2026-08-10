<script lang="ts">
	import { onMount } from 'svelte';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import ExcelJS from 'exceljs';
	import {
		comparendoApi,
		autoApi,
		businessApi,
		simitApi,
		ApiError,
		type Comparendo,
		type ComparendoDatos,
		type Auto,
		type BusinessLists,
		type InfoAgenteSimit,
		type ResultadoSincronizacion
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import OrdenComparendo from '$lib/components/reports/OrdenComparendo.svelte';
	import AvisoImpresion from '$lib/components/AvisoImpresion.svelte';
	import { imprimirDocumento } from '$lib/utils/imprimir';

	const sid = () => session.token ?? '';

	let comparendos = $state<Comparendo[]>([]);
	let autos = $state<Auto[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);

	// Filtros
	let busqueda = $state('');
	let estadoFiltro = $state('');
	let placaFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal crear/editar
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<ComparendoDatos>(defaultForm());
	let formError = $state('');

	// Marcar pagado / eliminar
	let pagandoId = $state<number | null>(null);
	let pagando = $state(false);
	let eliminarId = $state<number | null>(null);
	let eliminando = $state(false);
	
	let imprimirComparendo = $state<Comparendo | null>(null);

	// ── Agente SIMIT ──
	let agente = $state<InfoAgenteSimit | null>(null);
	let sincronizando = $state(false);
	let unlistenSimit: UnlistenFn | null = null;

	async function cargarAgente() {
		if (!haySesion()) return;
		try {
			agente = await simitApi.estado(sid());
		} catch (e) {
			// Sin backend Tauri (tests / standalone) o error transitorio → sin panel
			agente = null;
			if (!(e instanceof ApiError)) console.error('No se pudo consultar el estado del Agente SIMIT', e);
		}
	}

	async function sincronizarAhora() {
		if (sincronizando) return;
		sincronizando = true;
		try {
			const resultado = await simitApi.sincronizarAhora(sid());
			agente = { ...(agente ?? { habilitado: true, intervalHours: 2, ejecutando: false, ultimaSincronizacion: null, ultimoResultado: null, ultimoError: null }), ultimaSincronizacion: resultado.sincronizadoEn, ultimoResultado: resultado };
			if (resultado.insertados > 0) {
				toast.success(`Agente SIMIT: ${resultado.insertados} comparendo${resultado.insertados === 1 ? '' : 's'} nuevo${resultado.insertados === 1 ? '' : 's'} registrado${resultado.insertados === 1 ? '' : 's'}.`);
			} else {
				toast.success('Agente SIMIT: sincronización completada sin comparendos nuevos.');
			}
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo sincronizar con el SIMIT.');
		} finally {
			sincronizando = false;
		}
	}

	async function descargarExcelSimit() {
		const resultado = agente?.ultimoResultado;
		if (!resultado || resultado.registros.length === 0) {
			toast.error('No hay registros del SIMIT para exportar. Ejecuta primero una sincronización.');
			return;
		}
		try {
			const wb = new ExcelJS.Workbook();
			const ws = wb.addWorksheet('Comparendos SIMIT');
			ws.columns = Array.from({ length: 10 }, () => ({ width: 16 }));
			ws.mergeCells(1, 1, 1, 10);
			ws.getCell('A1').value = 'DINAMO RENT — REPORTE SIMIT';
			ws.getCell('A1').font = { bold: true, size: 14 };
			ws.mergeCells(2, 1, 2, 10);
			ws.getCell('A2').value = `Sincronización: ${new Date(resultado.sincronizadoEn).toLocaleString('es-CO')}`;
			ws.addRow([]);
			const header = ['Placa', 'N° Comparendo', 'Fecha', 'Hora', 'Tipo', 'Infracción', 'Organismo', 'Valor', 'Estado', 'Nuevo'];
			const headerRow = ws.addRow(header);
			headerRow.eachCell((cell) => {
				cell.font = { bold: true };
				cell.fill = { type: 'pattern', pattern: 'solid', fgColor: { argb: 'FFE8EAF6' } };
			});
			for (const r of resultado.registros) {
				ws.addRow([
					r.placa,
					r.numero ?? '',
					r.fechaInfraccion,
					r.horaInfraccion,
					r.esComparendo ? 'Comparendo' : 'Multa',
					r.descripcion,
					r.organismo,
					parseFloat(r.monto) || 0,
					r.estado,
					r.nuevo ? 'Sí' : 'No'
				]);
			}
			ws.addRow([]);
			ws.addRow(['Total pendiente', `$${resultado.totalPendiente}`]);
			const buffer = await wb.xlsx.writeBuffer();
			const blob = new Blob([buffer], {
				type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
			});
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `comparendos_simit_${resultado.sincronizadoEn.slice(0, 10)}.xlsx`;
			a.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			toast.error(e instanceof Error ? 'No se pudo generar el Excel: ' + e.message : 'No se pudo generar el Excel.');
		}
	}

	function defaultForm(): ComparendoDatos {
		const hoy = new Date();
		const iso = (d: Date) => d.toISOString().slice(0, 10);
		return {
			placa: '',
			fechaInfraccion: iso(hoy),
			horaInfraccion: '',
			monto: '',
			numeroComparendo: null,
			idRenta: null,
			idCliente: null,
			estado: 'Pendiente',
			observaciones: ''
		};
	}

	// ── Carga de datos ──
	async function cargar() {
		if (!haySesion()) return;
		loading = true;
		try {
			comparendos = await comparendoApi.listar(
				sid(),
				busqueda.trim() || undefined,
				placaFiltro || undefined,
				estadoFiltro || undefined
			);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los comparendos.');
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
		try {
			autos = await autoApi.listar(sid());
		} catch {
			autos = [];
		}
		await cargar();
		cargarAgente();
	});

	// El backend emite `simit-sync-complete` al terminar cada corrida
	// (programada cada 2 h o manual) → refrescar lista y panel.
	onMount(() => {
		let activo = true;
		listen<ResultadoSincronizacion>('simit-sync-complete', (evt) => {
			if (!activo) return;
			const r = evt.payload;
			agente = { ...(agente ?? { habilitado: true, intervalHours: 2, ejecutando: false, ultimaSincronizacion: null, ultimoResultado: null, ultimoError: null }), ultimaSincronizacion: r.sincronizadoEn, ultimoResultado: r };
			if (r.insertados > 0) {
				toast.success(`Agente SIMIT: ${r.insertados} comparendo${r.insertados === 1 ? '' : 's'} nuevo${r.insertados === 1 ? '' : 's'} registrado${r.insertados === 1 ? '' : 's'}.`);
			}
			cargar();
		})
			.then((u) => {
				if (activo) unlistenSimit = u;
				else u();
			})
			.catch(() => {
				// Sin runtime Tauri (tests / vite standalone): no hay evento de sincronización.
			});
		return () => {
			activo = false;
			unlistenSimit?.();
		};
	});

	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const _est = estadoFiltro;
		const _plac = placaFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	// ── CRUD ──
	function abrirNuevo() {
		form = defaultForm();
		editando = false;
		editandoId = null;
		formError = '';
		modalOpen = true;
	}

	function abrirEditar(c: Comparendo) {
		form = {
			placa: c.placa,
			fechaInfraccion: c.fechaInfraccion,
			horaInfraccion: c.horaInfraccion,
			monto: c.monto,
			// Conserva el número oficial (Agente SIMIT) al editar, para no romper la deduplicación
			numeroComparendo: c.numeroComparendo ?? null,
			idRenta: c.idRenta,
			idCliente: c.idCliente,
			estado: c.estado,
			observaciones: c.observaciones ?? ''
		};
		editando = true;
		editandoId = c.id;
		formError = '';
		modalOpen = true;
	}

	async function guardar() {
		formError = '';
		if (!form.placa.trim()) {
			formError = 'La placa es obligatoria.';
			return;
		}
		if (!form.fechaInfraccion) {
			formError = 'La fecha de la infracción es obligatoria.';
			return;
		}
		if (!form.horaInfraccion) {
			formError = 'La hora de la infracción es obligatoria.';
			return;
		}
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				await comparendoApi.actualizar(sid(), editandoId, form);
				toast.success(`Comparendo #${editandoId} actualizado.`);
			} else {
				const creado = await comparendoApi.crear(sid(), form);
				toast.success(`Comparendo #${creado.id} registrado.`);
			}
			modalOpen = false;
			await cargar();
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el comparendo.';
		} finally {
			guardando = false;
		}
	}

	async function confirmarPago() {
		if (pagandoId === null) return;
		pagando = true;
		try {
			await comparendoApi.marcarPagado(sid(), pagandoId);
			toast.success(`Comparendo #${pagandoId} marcado como pagado.`);
			pagandoId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo marcar el comparendo como pagado.');
			pagandoId = null;
		} finally {
			pagando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await comparendoApi.eliminar(sid(), eliminarId);
			toast.success('Comparendo eliminado.');
			eliminarId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el comparendo.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	// ── Presentación y Utilidades ──
	function cerrarImpresion() {
		imprimirComparendo = null;
	}

	function imprimir() {
		imprimirDocumento();
	}

	function estadoClases(estado: string): string {
		if (estado === 'Pagado') return 'bg-exito/10 text-exito border-exito/25';
		return 'bg-alerta/10 text-alerta border-alerta/25';
	}

	const tablaComparendos = $derived(comparendos as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'id', header: 'No.' },
		{ key: 'vehiculo', header: 'Vehículo' },
		{ key: 'fecha', header: 'Infracción' },
		{ key: 'monto', header: 'Monto' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'observaciones', header: 'Observaciones' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Comparendos — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Comparendos</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{comparendos.length} comparendo{comparendos.length === 1 ? '' : 's'} · multas de tránsito por vehículo
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Registrar Comparendo
		</button>
	</div>

	<!-- Agente SIMIT: consulta automática de comparendos por placa -->
	{#if agente}
		<div class="card p-4 space-y-3">
			<div class="flex flex-wrap items-center justify-between gap-3">
				<div class="flex items-center gap-3">
					<div class="w-10 h-10 rounded-xl bg-primary/10 text-primary flex items-center justify-center shrink-0">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 21a9.004 9.004 0 008.716-6.747M12 21a9.004 9.004 0 01-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 017.843 4.582M12 3a8.997 8.997 0 00-7.843 4.582m15.686 0A11.953 11.953 0 0112 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0121 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0112 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 013 12c0-1.605.42-3.113 1.157-4.418" /></svg>
					</div>
					<div>
						<p class="font-semibold text-text-primary">Agente SIMIT</p>
						<p class="text-xs text-text-secondary">
							Consulta automática de comparendos por placa · cada {agente.intervalHours} h
							{#if agente.ultimaSincronizacion}
								· última: {formatDate(agente.ultimaSincronizacion.slice(0, 10))} {agente.ultimaSincronizacion.slice(11, 16)}
								{:else}
								· aún sin sincronizar
							{/if}
						</p>
					</div>
				</div>
				<div class="flex items-center gap-2">
					{#if agente.ejecutando || sincronizando}
						<span class="inline-flex items-center gap-2 text-xs font-semibold text-primary">
							<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
							Consultando SIMIT...
						</span>
					{:else if !agente.habilitado}
						<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold text-text-secondary border-border">Deshabilitado en config.ini</span>
					{/if}
					<button class="btn-ghost" onclick={descargarExcelSimit} disabled={!agente.ultimoResultado || agente.ultimoResultado.registros.length === 0}>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" /></svg>
						Descargar Excel
					</button>
					<button class="btn-primary" onclick={sincronizarAhora} disabled={sincronizando || agente.ejecutando}>
						{#if sincronizando || agente.ejecutando}
							<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
							Sincronizando...
						{:else}
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" /></svg>
							Sincronizar ahora
						{/if}
					</button>
				</div>
			</div>

			{#if agente.ultimoResultado}
				{@const r = agente.ultimoResultado}
				<div class="flex flex-wrap items-center gap-x-5 gap-y-1.5 text-xs">
					<span class="text-text-secondary">
						Placas consultadas: <strong class="text-text-primary tabular-nums">{r.placasConsultadas}</strong>
					</span>
					<span class="text-text-secondary">
						Encontrados: <strong class="text-text-primary tabular-nums">{r.encontrados}</strong>
					</span>
					<span class="text-text-secondary">
						Nuevos en la BD: <strong class="text-exito tabular-nums">{r.insertados}</strong>
					</span>
					<span class="text-text-secondary">
						Ya registrados: <strong class="text-text-primary tabular-nums">{r.duplicados}</strong>
					</span>
					{#if r.errores.length > 0}
						<span class="text-text-secondary">
							Placas con error: <strong class="text-peligro tabular-nums">{r.errores.length}</strong>
						</span>
					{/if}
					<span class="text-text-secondary">
						Total pendiente: <strong class="text-alerta tabular-nums">{formatCOP(r.totalPendiente)}</strong>
					</span>
				</div>
				{#if r.reporteHtml}
					<p class="text-[11px] text-text-secondary font-mono truncate" title={r.reporteHtml}>
						Reporte HTML: {r.reporteHtml}
					</p>
				{/if}
				{#if r.errores.length > 0}
					<div class="rounded-lg bg-peligro/10 border border-peligro/25 px-3 py-2 text-[11px] text-peligro max-h-24 overflow-y-auto">
						{#each r.errores as e}
							<p>• {e.placa}: {e.error}</p>
						{/each}
					</div>
				{/if}
			{:else if agente.ultimoError}
				<p class="text-xs text-peligro">
					Último error de sincronización: {agente.ultimoError}
				</p>
			{/if}
		</div>
	{/if}

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por placa u observaciones..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={estadoFiltro} aria-label="Filtrar por estado">
			<option value="">Todos los estados</option>
			<option value="Pendiente">Pendiente</option>
			<option value="Pagado">Pagado</option>
		</select>
		<select class="input w-auto" bind:value={placaFiltro} aria-label="Filtrar por placa">
			<option value="">Todas las placas</option>
			{#each autos as a}
				<option value={a.placa}>{a.placa} · {a.marca} {a.modelo}</option>
			{/each}
		</select>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando comparendos...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaComparendos}
			emptyTitle="No hay comparendos"
			emptyDescription="Registra el primer comparendo con el botón «Registrar Comparendo»."
			emptyIcon="check"
		>
			{#snippet children(col, item)}
				{@const c = item as unknown as Comparendo}
				{#if col.key === 'id'}
					<span class="font-bold text-primary tabular-nums">#{String(c.id).padStart(4, '0')}</span>
				{:else if col.key === 'vehiculo'}
					<div>
						<p class="text-text-primary truncate max-w-[160px]">{c.vehiculo || '—'}</p>
						<p class="text-xs text-text-secondary font-mono">{c.placa}</p>
					</div>
				{:else if col.key === 'fecha'}
					<div class="whitespace-nowrap">
						<p class="text-text-primary tabular-nums">{formatDate(c.fechaInfraccion)}</p>
						<p class="text-xs text-text-secondary">{c.horaInfraccion}</p>
					</div>
				{:else if col.key === 'monto'}
					<p class="font-bold text-text-primary tabular-nums text-right whitespace-nowrap">{formatCOP(c.monto)}</p>
				{:else if col.key === 'estado'}
					<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {estadoClases(c.estado)}">
						<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
						{c.estado}
					</span>
				{:else if col.key === 'observaciones'}
					<p class="text-sm text-text-secondary truncate max-w-[220px]">{c.observaciones || '—'}</p>
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Imprimir notificación"
							onclick={() => (imprimirComparendo = c)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
						</button>
						{#if c.estado !== 'Pagado'}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-exito hover:bg-exito/10 transition-colors"
								title="Marcar como pagado"
								onclick={() => (pagandoId = c.id)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
							</button>
						{/if}
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
							onclick={() => (eliminarId = c.id)}
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
	title={editando ? `Editar comparendo #${editandoId}` : 'Registrar comparendo'}
	subtitle={editando ? 'Modifica los datos y guarda los cambios.' : 'Registra una multa de tránsito para un vehículo.'}
	onClose={() => (modalOpen = false)}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Placa" required>
				<select class="input" bind:value={form.placa}>
					<option value="">— Seleccionar vehículo —</option>
					{#each autos as a}
						<option value={a.placa}>{a.placa} · {a.marca} {a.modelo}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Monto (COP)" required>
				<input class="input" inputmode="decimal" placeholder="Ej: 580000" bind:value={form.monto} />
			</FormField>
			<FormField label="Fecha de la infracción" required>
				<input class="input" type="date" bind:value={form.fechaInfraccion} />
			</FormField>
			<FormField label="Hora de la infracción" required>
				<input class="input" type="time" placeholder="HH:MM" bind:value={form.horaInfraccion} />
			</FormField>
			<FormField label="Estado">
				<select class="input" bind:value={form.estado}>
					<option value="Pendiente">Pendiente</option>
					<option value="Pagado">Pagado</option>
				</select>
			</FormField>
			<FormField label="Observaciones">
				<input class="input" placeholder="Ej: Exceso de velocidad, foto-detección..." bind:value={form.observaciones} maxlength="2000" />
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
		<button class="btn-primary" onclick={guardar} disabled={guardando}>
			{#if guardando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				{editando ? 'Guardar cambios' : 'Registrar comparendo'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Confirmación de pago -->
<ConfirmDialog
	open={pagandoId !== null}
	title="Marcar comparendo como pagado"
	message="¿Confirmas que este comparendo ya fue pagado?"
	confirmLabel="Marcar pagado"
	loading={pagando}
	onConfirm={confirmarPago}
	onCancel={() => (pagandoId = null)}
/>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar comparendo"
	message="¿Seguro que deseas eliminar este comparendo? Esta acción no se puede deshacer."
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>

<!-- Modal orden imprimible -->
<Modal
	open={imprimirComparendo !== null}
	title={imprimirComparendo ? `Notificación de comparendo #${String(imprimirComparendo.id).padStart(4, '0')}` : ''}
	subtitle="Vista previa del documento. Al imprimir solo se muestra la orden."
	onClose={cerrarImpresion}
	width="max-w-3xl"
>
	{#snippet children()}
		{#if imprimirComparendo}
			<AvisoImpresion />
			<OrdenComparendo comparendo={imprimirComparendo} />
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost print-hidden" onclick={cerrarImpresion}>Cerrar</button>
		<button class="btn-primary print-hidden" onclick={imprimir}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
			Imprimir documento
		</button>
	{/snippet}
</Modal>

