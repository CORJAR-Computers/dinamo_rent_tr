<script lang="ts">
	import { onMount } from 'svelte';
	import {
		reservaApi,
		clienteApi,
		autoApi,
		businessApi,
		ApiError,
		type Reserva,
		type ReservaDatos,
		type ClienteConPii,
		type Auto,
		type BusinessLists
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { calcularDiasHoras } from '$lib/utils/calcularDiasHoras';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import SearchSelect, { type SearchSelectOpcion } from '$lib/components/SearchSelect.svelte';
	import ClienteFormModal from '$lib/components/ClienteFormModal.svelte';
	import OrdenReserva from '$lib/components/reports/OrdenReserva.svelte';
	import AvisoImpresion from '$lib/components/AvisoImpresion.svelte';
	import { imprimirDocumento } from '$lib/utils/imprimir';

	const sid = () => session.token ?? '';

	let reservas = $state<Reserva[]>([]);
	let proximas = $state<Reserva[]>([]);
	let clientes = $state<ClienteConPii[]>([]);
	let autos = $state<Auto[]>([]);
	let lists = $state<BusinessLists | null>(null);

	// ¿El rol actual puede eliminar registros? (roles_con_eliminar de config.ini)
	const puedeEliminar = $derived(
		(lists?.rolesConEliminar ?? ['Administrador', 'Supervisor']).includes(session.user?.rol ?? '')
	);
	let loading = $state(true);

	// Filtros
	let busqueda = $state('');
	let estadoFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal crear/editar
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<ReservaDatos>(defaultForm());
	let formError = $state('');

	// Modal cliente embebido (crear cliente sin salir de la reserva)
	let clienteModalOpen = $state(false);

	// Modal imprimir
	let imprimirReserva = $state<Reserva | null>(null);

	// Cancelar / eliminar
	let cancelarId = $state<number | null>(null);
	let cancelarNombre = $state('');
	let cancelando = $state(false);
	let eliminarId = $state<number | null>(null);
	let eliminando = $state(false);

	function defaultForm(): ReservaDatos {
		const hoy = new Date();
		const maniana = new Date(hoy.getTime() + 86400000);
		const iso = (d: Date) => d.toISOString().slice(0, 10);
		return {
			idCliente: null,
			nombreCliente: '',
			nacionalidad: '',
			categoriaVehiculo: 'Automóvil',
			placaAsignada: '',
			fechaRecogida: iso(hoy),
			horaRecogida: '',
			ubicacionRecogida: '',
			fechaRetorno: iso(maniana),
			horaRetorno: '',
			ubicacionRetorno: '',
			diasCalculados: 1,
			horasExtras: 0,
			valorDia: '',
			valorHoraAdic: '',
			abono: '',
			total: '',
			observaciones: '',
			estado: 'Confirmada'
		};
	}

	// ── Calculadora en vivo ──
	const totalCalc = $derived(
		(parseFloat(form.valorDia) || 0) * form.diasCalculados +
			(parseFloat(form.valorHoraAdic) || 0) * form.horasExtras
	);
	const saldoCalc = $derived(Math.max(0, totalCalc - (parseFloat(form.abono) || 0)));

	function recalcularDias() {
		// Regla unificada (espejo del cierre de renta): cada 24 h = 1 día;
		// excedente ≤ 3 h → horas extras; excedente > 3 h → día completo.
		const { dias, horas } = calcularDiasHoras(
			form.fechaRecogida,
			form.horaRecogida ?? '',
			form.fechaRetorno,
			form.horaRetorno ?? ''
		);
		form.diasCalculados = dias;
		form.horasExtras = horas;
	}

	function onClienteChange(v: string) {
		form.idCliente = v === '' ? null : Number(v);
		const c = clientes.find((x) => x.cliente.id === form.idCliente);
		form.nombreCliente = c?.cliente.nombreCompleto ?? '';
		form.nacionalidad = c?.cliente.nacionalidad ?? '';
	}

	// Opciones para el combo de cliente (filtra por nombre y número de documento).
	const opcionesClientes = $derived<SearchSelectOpcion[]>(
		clientes.map((c) => ({
			value: String(c.cliente.id),
			label: c.cliente.nombreCompleto,
			sub: [c.cliente.tipoDoc ?? '', c.cliente.noDoc ?? ''].filter(Boolean).join(' ').trim()
		}))
	);

	// Cliente creado desde el modal embebido: autoseleccionarlo en la reserva
	async function onNuevoClienteGuardado(r: ClienteConPii) {
		clienteModalOpen = false;
		const c = r.cliente;
		form.idCliente = c.id;
		form.nombreCliente = c.nombreCompleto;
		form.nacionalidad = c.nacionalidad ?? '';
		toast.success(`Cliente ${c.nombreCompleto} creado y seleccionado.`);
		try {
			clientes = await clienteApi.listar(sid());
		} catch {
			/* la selección ya quedó aplicada; la lista se refresca en la próxima carga */
		}
	}

	const autosCategoria = $derived(
		form.categoriaVehiculo ? autos.filter((a) => a.tipo === form.categoriaVehiculo) : autos
	);

	// Opciones para el combo de vehículo (filtra por placa, marca, modelo, tipo o color).
	const opcionesAutos = $derived<SearchSelectOpcion[]>(
		autosCategoria.map((a) => ({
			value: a.placa,
			label: `${a.placa} · ${a.marca} ${a.modelo}`,
			sub: [a.tipo ?? '', a.color ?? ''].filter(Boolean).join(' ').trim()
		}))
	);

	// ── Carga de datos ──
	async function cargar() {
		if (!haySesion()) return;
		loading = true;
		try {
			reservas = await reservaApi.listar(sid(), busqueda.trim() || undefined, estadoFiltro || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar las reservas.');
		} finally {
			loading = false;
		}
	}

	async function cargarProximas() {
		if (!haySesion()) return;
		try {
			proximas = await reservaApi.proximas(sid(), 5);
		} catch {
			proximas = [];
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
			clientes = await clienteApi.listar(sid());
		} catch {
			clientes = [];
		}
		try {
			autos = await autoApi.listar(sid());
		} catch {
			autos = [];
		}
		await cargarProximas();
		// La carga inicial de reservas la dispara el $effect de filtros (una sola vez)
	});

	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const _est = estadoFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			cargar();
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

	function abrirEditar(r: Reserva) {
		form = {
			idCliente: r.idCliente,
			nombreCliente: r.nombreCliente,
			nacionalidad: r.nacionalidad ?? '',
			categoriaVehiculo: r.categoriaVehiculo ?? 'Automóvil',
			placaAsignada: r.placaAsignada ?? '',
			fechaRecogida: r.fechaRecogida,
			horaRecogida: r.horaRecogida ?? '',
			ubicacionRecogida: r.ubicacionRecogida ?? '',
			fechaRetorno: r.fechaRetorno,
			horaRetorno: r.horaRetorno ?? '',
			ubicacionRetorno: r.ubicacionRetorno ?? '',
			diasCalculados: r.diasCalculados,
			horasExtras: r.horasExtras,
			valorDia: r.valorDia,
			valorHoraAdic: r.valorHoraAdic,
			abono: r.abono,
			total: r.total,
			observaciones: r.observaciones ?? '',
			estado: r.estado
		};
		editando = true;
		editandoId = r.id;
		formError = '';
		modalOpen = true;
	}

	async function guardar() {
		formError = '';
		if (!form.nombreCliente.trim()) {
			formError = 'El nombre del cliente es obligatorio.';
			return;
		}
		if (!form.fechaRecogida || !form.fechaRetorno) {
			formError = 'Las fechas de recogida y retorno son obligatorias.';
			return;
		}
		if (form.fechaRetorno < form.fechaRecogida) {
			formError = 'La fecha de retorno no puede ser anterior a la recogida.';
			return;
		}
		// El backend recalcula el total, pero se envía coherente con la calculadora
		form.total = totalCalc.toFixed(2);
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				await reservaApi.actualizar(sid(), editandoId, form);
				toast.success(`Reserva #${editandoId} actualizada.`);
			} else {
				const creada = await reservaApi.crear(sid(), form);
				toast.success(`Reserva #${creada.id} creada.`);
			}
			modalOpen = false;
			await Promise.all([cargar(), cargarProximas()]);
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar la reserva.';
		} finally {
			guardando = false;
		}
	}

	async function confirmarCancelar() {
		if (cancelarId === null) return;
		cancelando = true;
		try {
			const r = await reservaApi.cancelar(sid(), cancelarId);
			toast.success(
				r.cancelada
					? `Reserva #${cancelarId} cancelada.`
					: `La reserva #${cancelarId} ya estaba cancelada.`
			);
			cancelarId = null;
			await Promise.all([cargar(), cargarProximas()]);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo cancelar la reserva.');
			cancelarId = null;
		} finally {
			cancelando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await reservaApi.eliminar(sid(), eliminarId);
			toast.success('Reserva eliminada.');
			eliminarId = null;
			await Promise.all([cargar(), cargarProximas()]);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar la reserva.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	// ── Impresión ──
	function abrirImprimir(r: Reserva) {
		imprimirReserva = r;
	}

	function imprimir() {
		imprimirDocumento();
	}

	function cerrarImpresion() {
		imprimirReserva = null;
		if (typeof document !== 'undefined') {
			document.body.classList.remove('printing');
		}
	}

	// ── Presentación ──
	function estadoClases(estado: string): string {
		if (estado === 'Confirmada') return 'bg-primary/10 text-primary border-primary/25';
		if (estado === 'Pendiente') return 'bg-alerta/10 text-alerta border-alerta/25';
		if (estado === 'Cancelada') return 'bg-peligro/10 text-peligro border-peligro/25';
		if (estado === 'Completada') return 'bg-estado-activo/10 text-estado-activo border-estado-activo/25';
		return 'bg-text-secondary/10 text-text-secondary border-text-secondary/25';
	}

	function fmtHora(h: string | null): string {
		if (!h) return '—';
		const [hh, mm] = h.split(':');
		return `${hh}:${mm}`;
	}

	const tablaReservas = $derived(reservas as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'id', header: 'No.' },
		{ key: 'cliente', header: 'Cliente' },
		{ key: 'vehiculo', header: 'Vehículo' },
		{ key: 'recogida', header: 'Recogida' },
		{ key: 'retorno', header: 'Retorno' },
		{ key: 'financiero', header: 'Total / Abono' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Reservas — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Reservas</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{reservas.length} reserva{reservas.length === 1 ? '' : 's'} · orden imprimible incluida
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Nueva Reserva
		</button>
	</div>

	<!-- Próximas reservas -->
	{#if proximas.length > 0}
		<div class="rounded-xl border border-primary/20 bg-primary/5 px-4 py-3">
			<div class="flex items-center gap-2 mb-2">
				<span class="w-2 h-2 rounded-full bg-primary animate-pulse"></span>
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary">
					Próximas entregas ({proximas.length})
				</h3>
			</div>
			<div class="flex flex-wrap gap-2">
				{#each proximas as p}
					<span
						class="inline-flex items-center gap-1.5 rounded-lg border border-primary/25 bg-surface px-2.5 py-1.5 text-[11px] font-semibold transition-transform hover:scale-[1.02] cursor-default"
						title={`${p.nombreCliente} · ${p.categoriaVehiculo ?? ''} ${p.placaAsignada ? `· ${p.placaAsignada}` : ''}`}
					>
						<span class="w-1.5 h-1.5 rounded-full bg-current text-primary"></span>
						{formatDate(p.fechaRecogida)}{p.horaRecogida ? ` ${fmtHora(p.horaRecogida)}` : ''} · {p.nombreCliente}
					</span>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por cliente, placa o nacionalidad..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={estadoFiltro} aria-label="Filtrar por estado">
			<option value="">Todos los estados</option>
			{#each (lists?.estadosReserva ?? ['Pendiente', 'Confirmada', 'Cancelada', 'Completada']) as est}
				<option value={est}>{est}</option>
			{/each}
		</select>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando reservas...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaReservas}
			emptyTitle="No hay reservas"
			emptyDescription="Crea la primera reserva con el botón «Nueva Reserva»."
			emptyIcon="calendar"
		>
			{#snippet children(col, item)}
				{@const r = item as unknown as Reserva}
				{#if col.key === 'id'}
					<span class="font-bold text-primary tabular-nums">#{String(r.id).padStart(4, '0')}</span>
				{:else if col.key === 'cliente'}
					<div class="max-w-[200px]">
						<p class="font-semibold text-text-primary truncate">{r.nombreCliente}</p>
						{#if r.nacionalidad}
							<p class="text-xs text-text-secondary truncate">{r.nacionalidad}</p>
						{/if}
					</div>
				{:else if col.key === 'vehiculo'}
					<div>
						<p class="text-text-primary">{r.categoriaVehiculo || '—'}</p>
						<p class="text-xs text-text-secondary font-mono">{r.placaAsignada || 'Sin asignar'}</p>
					</div>
				{:else if col.key === 'recogida'}
					<div class="whitespace-nowrap">
						<p class="text-text-primary tabular-nums">{formatDate(r.fechaRecogida)} <span class="text-text-secondary">{fmtHora(r.horaRecogida)}</span></p>
						{#if r.ubicacionRecogida}
							<p class="text-xs text-text-secondary truncate max-w-[160px]">{r.ubicacionRecogida}</p>
						{/if}
					</div>
				{:else if col.key === 'retorno'}
					<div class="whitespace-nowrap">
						<p class="text-text-primary tabular-nums">{formatDate(r.fechaRetorno)} <span class="text-text-secondary">{fmtHora(r.horaRetorno)}</span></p>
						<p class="text-xs text-text-secondary">
							{r.diasCalculados} día{r.diasCalculados === 1 ? '' : 's'}{r.horasExtras > 0 ? ` + ${r.horasExtras}h` : ''}
						</p>
					</div>
				{:else if col.key === 'financiero'}
					<div class="text-right whitespace-nowrap">
						<p class="font-bold text-text-primary tabular-nums">{formatCOP(r.total)}</p>
						<p class="text-xs text-text-secondary tabular-nums">Abono: {formatCOP(r.abono)}</p>
					</div>
				{:else if col.key === 'estado'}
					<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {estadoClases(r.estado)}">
						<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
						{r.estado}
					</span>
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Imprimir orden de reserva"
							onclick={() => abrirImprimir(r)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(r)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						{#if r.estado !== 'Cancelada' && r.estado !== 'Completada'}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-alerta hover:bg-alerta/10 transition-colors"
								title="Cancelar reserva"
								onclick={() => {
									cancelarId = r.id;
									cancelarNombre = r.nombreCliente;
								}}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9.75 9.75l4.5 4.5m0-4.5l-4.5 4.5M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
							</button>
						{/if}
						{#if puedeEliminar}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
								title="Eliminar"
								onclick={() => (eliminarId = r.id)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" /></svg>
							</button>
						{/if}
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
	title={editando ? `Editar reserva #${editandoId}` : 'Nueva reserva'}
	subtitle={editando ? 'Modifica los datos y guarda los cambios.' : 'Registra una reserva para un cliente.'}
	onClose={() => (modalOpen = false)}
	width="max-w-6xl"
	fullHeight
	rawBody
>
	{#snippet children()}
		<div class="flex grow min-h-0">
			<!-- ── Panel izquierdo: campos (scrollable si es necesario) ── -->
			<div class="flex-1 min-w-0 overflow-y-auto din-scroll px-5 py-4">

				{#if formError}
					<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
				{/if}

				<!-- ── 1. Cliente ── -->
				<div class="flex items-center gap-2 mb-2.5">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">1</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Cliente</h3>
				</div>
				<div class="grid grid-cols-2 gap-x-3 mb-3">
					<div class="col-span-2">
						<div class="flex items-end gap-2">
							<SearchSelect
								class="grow"
								label="Cliente registrado"
								hint="Opcional: busca por nombre o número de documento; se autocompleta el resto."
								dense
								value={form.idCliente === null ? '' : String(form.idCliente)}
								opciones={opcionesClientes}
								onchange={onClienteChange}
								placeholder="Buscar por nombre o documento…"
								vacioLabel="— Sin cliente registrado —"
							/>
							<button
								type="button"
								class="mb-3 shrink-0 inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs font-semibold border border-primary text-primary hover:bg-primary hover:text-white transition-colors"
								onclick={() => (clienteModalOpen = true)}
								title="Crear nuevo cliente"
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
								<span class="hidden xl:inline">Nuevo</span>
							</button>
						</div>
					</div>
					<FormField label="Nombre del cliente" required dense>
						<input class="input" placeholder="Nombre para la reserva" bind:value={form.nombreCliente} maxlength="200" />
					</FormField>
					<FormField label="Nacionalidad" dense>
						<input class="input" placeholder="Ej: Colombiana" bind:value={form.nacionalidad} maxlength="80" />
					</FormField>
				</div>

				<!-- ── 2. Vehículo ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">2</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Vehículo</h3>
				</div>
				<div class="grid grid-cols-2 gap-x-3 mb-3">
					<FormField label="Categoría" dense>
						<select class="input" bind:value={form.categoriaVehiculo}>
							{#each (lists?.tiposAuto ?? ['Automóvil', 'Camioneta', 'Van', 'Lujo', 'Moto']) as t}
								<option value={t}>{t}</option>
							{/each}
						</select>
					</FormField>
					<SearchSelect
						label="Placa asignada"
						dense
						value={form.placaAsignada ?? ''}
						opciones={opcionesAutos}
						onchange={(v) => (form.placaAsignada = v)}
						placeholder="Buscar placa, marca o modelo…"
						vacioLabel="— Sin asignar —"
					/>
				</div>

				<!-- ── 3. Itinerario ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">3</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Itinerario</h3>
				</div>
				<div class="grid grid-cols-2 gap-x-3 mb-3">
					<FormField label="Fecha de recogida" required dense>
						<input class="input" type="date" bind:value={form.fechaRecogida} onchange={recalcularDias} />
					</FormField>
					<FormField label="Hora de recogida" dense>
						<input class="input" type="time" bind:value={form.horaRecogida} onchange={recalcularDias} />
					</FormField>
					<FormField label="Lugar de recogida" dense>
						<input class="input" placeholder="Ej: Aeropuerto, oficina..." bind:value={form.ubicacionRecogida} maxlength="200" />
					</FormField>
					<FormField label="Lugar de retorno" dense>
						<input class="input" placeholder="Ej: Aeropuerto, oficina..." bind:value={form.ubicacionRetorno} maxlength="200" />
					</FormField>
					<FormField label="Fecha de retorno" required dense>
						<input class="input" type="date" bind:value={form.fechaRetorno} onchange={recalcularDias} />
					</FormField>
					<FormField label="Hora de retorno" dense>
						<input class="input" type="time" bind:value={form.horaRetorno} onchange={recalcularDias} />
					</FormField>
				</div>

				<!-- ── 4. Tarifas y totales ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">4</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Tarifas y totales</h3>
				</div>
				<div class="grid grid-cols-2 gap-x-3 mb-3">
					<FormField label="Valor por día" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="150000" bind:value={form.valorDia} />
					</FormField>
					<FormField label="Valor hora adicional" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="10000" bind:value={form.valorHoraAdic} />
					</FormField>
					<FormField label="Días calculados" hint="Auto desde fechas" dense>
						<input class="input" type="number" min="0" step="1" bind:value={form.diasCalculados} />
					</FormField>
					<FormField label="Horas extras" dense>
						<input class="input" type="number" min="0" step="1" bind:value={form.horasExtras} />
					</FormField>
					<FormField label="Abono" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="100000" bind:value={form.abono} />
					</FormField>
					<FormField label="Estado" dense>
						<select class="input" bind:value={form.estado}>
							{#each (lists?.estadosReserva ?? ['Pendiente', 'Confirmada', 'Cancelada', 'Completada']) as e}
								<option value={e}>{e}</option>
							{/each}
						</select>
					</FormField>
				</div>
			</div>

			<!-- ── Panel derecho: resumen + observaciones + acciones (sticky) ── -->
			<div class="w-72 xl:w-80 shrink-0 border-l border-border bg-alt-row/40 flex flex-col">
				<!-- Resumen en vivo (siempre visible) -->
				<div class="px-4 py-3 border-b border-border">
					<div class="flex items-center gap-2 mb-2.5">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456z" /></svg>
						<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Resumen en vivo</h3>
					</div>
					<!-- Total destacado -->
					<div class="rounded-lg bg-gradient-to-br from-primary to-primary-hover px-3 py-2.5 text-white mb-2">
						<p class="text-[10px] uppercase tracking-wide opacity-80 font-semibold">Total estimado</p>
						<p class="text-xl font-black tabular-nums leading-tight">{formatCOP(totalCalc)}</p>
						<p class="text-[10px] opacity-80 mt-0.5">
							{form.diasCalculados} día{form.diasCalculados === 1 ? '' : 's'}{form.horasExtras > 0 ? ` + ${form.horasExtras}h` : ''}
						</p>
					</div>
					<!-- Desglose compacto -->
					<div class="space-y-1 text-xs">
						<div class="flex justify-between">
							<span class="text-text-secondary">Abono</span>
							<span class="font-semibold text-text-primary tabular-nums">{formatCOP(form.abono)}</span>
						</div>
						<div class="flex justify-between pt-1 border-t border-border">
							<span class="text-text-secondary font-semibold">Saldo</span>
							<span class="font-bold text-exito tabular-nums text-sm">{formatCOP(saldoCalc)}</span>
						</div>
					</div>
				</div>

				<!-- Observaciones -->
				<div class="px-4 py-3 grow flex flex-col min-h-0">
					<span class="label flex items-center gap-1.5">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15 10.5a3 3 0 11-6 0 3 3 0 016 0z" /><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 10.5c0 7.142-7.5 11.25-7.5 11.25S4.5 17.642 4.5 10.5a7.5 7.5 0 1115 0z" /></svg>
						Observaciones
					</span>
					<textarea
						class="input flex-1 min-h-[60px] resize-none text-xs"
						placeholder="Aparecen en la orden imprimible…"
						bind:value={form.observaciones}
						maxlength="2000"
					></textarea>
					<p class="text-[10px] text-text-secondary/70 mt-1">{(form.observaciones ?? '').length}/2000</p>
				</div>

				<!-- Acciones -->
				<div class="px-4 py-3 border-t border-border bg-surface/50 flex flex-col gap-2">
					<button class="btn-primary w-full" onclick={guardar} disabled={guardando}>
						{#if guardando}
							<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
							Guardando...
						{:else}
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" /></svg>
							{editando ? 'Guardar cambios' : 'Crear reserva'}
						{/if}
					</button>
					<button class="btn-ghost w-full" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
				</div>
			</div>
		</div>
	{/snippet}
</Modal>

<!-- Modal orden imprimible -->
<Modal
	open={imprimirReserva !== null}
	title={imprimirReserva ? `Orden de reserva #${String(imprimirReserva.id).padStart(4, '0')}` : ''}
	subtitle="Vista previa del documento. Al imprimir solo se muestra la orden."
	onClose={cerrarImpresion}
	width="max-w-3xl"
>
	{#snippet children()}
		{#if imprimirReserva}
			<AvisoImpresion />
			<OrdenReserva reserva={imprimirReserva} />
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost print-hidden" onclick={cerrarImpresion}>Cerrar</button>
		<button class="btn-primary print-hidden" onclick={imprimir}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
			Imprimir orden
		</button>
	{/snippet}
</Modal>

<!-- Modal cliente embebido: crear un cliente nuevo sin salir del formulario de reserva -->
<ClienteFormModal
	open={clienteModalOpen}
	editando={null}
	lists={lists}
	clientes={clientes}
	onClose={() => (clienteModalOpen = false)}
	onGuardado={onNuevoClienteGuardado}
/>

<!-- Confirmación de cancelación -->
<ConfirmDialog
	open={cancelarId !== null}
	title="Cancelar reserva"
	message={`¿Seguro que deseas cancelar la reserva de ${cancelarNombre}? Podrás verla con estado «Cancelada».`}
	confirmLabel="Cancelar reserva"
	loading={cancelando}
	onConfirm={confirmarCancelar}
	onCancel={() => (cancelarId = null)}
/>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar reserva"
	message="¿Seguro que deseas eliminar esta reserva? Esta acción no se puede deshacer."
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>
