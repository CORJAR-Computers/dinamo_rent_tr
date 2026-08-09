<script lang="ts">
	import { onMount } from 'svelte';
	import {
		rentaApi,
		clienteApi,
		autoApi,
		businessApi,
		ApiError,
		type Renta,
		type RentaDatos,
		type RentaCierreDatos,
		type PagoDatos,
		type InspeccionDatos,
		type ClienteConPii,
		type Auto,
		type BusinessLists
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import ClienteFormModal from '$lib/components/ClienteFormModal.svelte';
	import OrdenRenta from '$lib/components/reports/OrdenRenta.svelte';
	import ContratoRenta from '$lib/components/reports/ContratoRenta.svelte';
	import AvisoImpresion from '$lib/components/AvisoImpresion.svelte';
	import { imprimirDocumento } from '$lib/utils/imprimir';

	const sid = () => session.token ?? '';

	let rentas = $state<Renta[]>([]);
	let clientes = $state<ClienteConPii[]>([]);
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
	let form = $state<RentaDatos>(defaultForm());
	let formError = $state('');

	// Modal cliente embebido (crear cliente sin salir de la renta)
	let clienteModalOpen = $state(false);

	// Modal cierre
	let cerrandoId = $state<number | null>(null);
	let cierre = $state<RentaCierreDatos>(defaultCierre());
	let cerrando = $state(false);
	let cierreError = $state('');

	// Modal pago
	let pagandoId = $state<number | null>(null);
	let pago = $state<PagoDatos>(defaultPago());
	let pagoError = $state('');
	let guardandoPago = $state(false);

	// Modal inspección
	let inspeccionandoId = $state<number | null>(null);
	let inspeccionTipo = $state<'Salida' | 'Entrada'>('Salida');
	let inspeccion = $state<InspeccionDatos>(defaultInspeccion('Salida'));
	let inspeccionError = $state('');
	let guardandoInspeccion = $state(false);

	// Modal imprimir
	let imprimirRenta = $state<Renta | null>(null);
	// Modal contrato (documento independiente, papel Carta)
	let imprimirContrato = $state<Renta | null>(null);

	// Cancelar / eliminar
	let cancelarId = $state<number | null>(null);
	let cancelarNombre = $state('');
	let cancelando = $state(false);
	let eliminarId = $state<number | null>(null);
	let eliminando = $state(false);

	function defaultForm(): RentaDatos {
		const hoy = new Date();
		const maniana = new Date(hoy.getTime() + 86400000);
		const iso = (d: Date) => d.toISOString().slice(0, 10);
		return {
			placa: null,
			idCliente: null,
			nombreCliente: '',
			noLicencia: '',
			nacionalidad: '',
			fechaRecogida: iso(hoy),
			horaRecogida: '',
			ubicacionRecogida: '',
			fechaRetorno: iso(maniana),
			horaRetorno: '',
			ubicacionRetorno: '',
			diasCalculados: 1,
			horasExtras: 0,
			valorDia: '',
			valorHoraExtra: '',
			valorDiaExtra: '0',
			costoLavado: '0',
			costoSilla: '0',
			costoRetorno: '0',
			costoDomicilio: '0',
			costoCables: '0',
			costoInversor: '0',
			descuento: '0',
			abono: '0',
			observaciones: '',
			kmSalida: '',
			tanqueSalida: 'Lleno',
			idReserva: null
		};
	}

	function defaultCierre(): RentaCierreDatos {
		const hoy = new Date();
		const iso = (d: Date) => d.toISOString().slice(0, 10);
		return {
			fechaDevolucionReal: iso(hoy),
			horaDevolucionReal: '',
			kmFinal: '',
			tanqueFinal: 'Lleno',
			diasCalculados: null,
			horasExtras: null,
			valorDia: '',
			valorHoraExtra: '',
			descuento: '',
			observaciones: ''
		};
	}

	function defaultPago(): PagoDatos {
		return { monto: '', metodoPago: 'Efectivo', concepto: 'Abono renta', observaciones: '' };
	}

	function defaultInspeccion(tipo: 'Salida' | 'Entrada'): InspeccionDatos {
		return {
			tipo,
			kilometraje: '',
			nivelGasolina: 'Lleno',
			limpieza: 'Limpio',
			tieneRepuesto: true,
			tieneGatoCruceta: true,
			tieneKitCarretera: true,
			tieneDocumentos: true,
			danosCarroceria: '',
			observaciones: ''
		};
	}

	// ── Calculadora en vivo (espejo del cálculo del backend, sin IVA para vista previa) ──
	const brutoCalc = $derived(
		(parseFloat(form.valorDia) || 0) * form.diasCalculados +
			(parseFloat(form.valorHoraExtra) || 0) * form.horasExtras
	);
	const extrasCalc = $derived(
		['valorDiaExtra', 'costoLavado', 'costoSilla', 'costoRetorno', 'costoDomicilio', 'costoCables', 'costoInversor']
			.reduce((acc, k) => acc + (parseFloat(form[k as keyof RentaDatos] as string) || 0), 0)
	);
	const subtotalCalc = $derived(Math.max(0, brutoCalc + extrasCalc - (parseFloat(form.descuento) || 0)));
	const totalCalc = $derived(subtotalCalc);
	const saldoCalc = $derived(Math.max(0, totalCalc - (parseFloat(form.abono) || 0)));

	function recalcularDias() {
		if (!form.fechaRecogida || !form.fechaRetorno) {
			form.diasCalculados = 0;
			return;
		}
		const a = new Date(form.fechaRecogida + 'T00:00:00');
		const b = new Date(form.fechaRetorno + 'T00:00:00');
		const d = Math.round((b.getTime() - a.getTime()) / 86_400_000);
		form.diasCalculados = Math.max(0, d);
	}

	function onClienteChange(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		form.idCliente = v === '' ? null : Number(v);
		const c = clientes.find((x) => x.cliente.id === form.idCliente);
		form.nombreCliente = c?.cliente.nombreCompleto ?? '';
		form.nacionalidad = c?.cliente.nacionalidad ?? '';
		form.noLicencia = c?.cliente.noLicencia ?? '';
	}

	// Cliente creado desde el modal embebido: autoseleccionarlo en la renta
	async function onNuevoClienteGuardado(r: ClienteConPii) {
		clienteModalOpen = false;
		const c = r.cliente;
		form.idCliente = c.id;
		form.nombreCliente = c.nombreCompleto;
		form.nacionalidad = c.nacionalidad ?? '';
		form.noLicencia = c.noLicencia ?? '';
		toast.success(`Cliente ${c.nombreCompleto} creado y seleccionado.`);
		try {
			clientes = await clienteApi.listar(sid());
		} catch {
			/* la selección ya quedó aplicada; la lista se refresca en la próxima carga */
		}
	}

	function onPlacaChange(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		form.placa = v === '' ? null : v;
		const a = autos.find((x) => x.placa === v);
		if (a && !form.kmSalida) form.kmSalida = String(a.kilometraje || '');
	}

	// ── Carga de datos ──
	async function cargar() {
		if (!haySesion()) return;
		loading = true;
		try {
			rentas = await rentaApi.listar(
				sid(),
				busqueda.trim() || undefined,
				estadoFiltro || undefined,
				placaFiltro || undefined
			);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar las rentas.');
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
			clientes = await clienteApi.listar(sid());
		} catch {
			clientes = [];
		}
		try {
			autos = await autoApi.listar(sid());
		} catch {
			autos = [];
		}
		await cargar();
	});

	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const est = estadoFiltro;
		const plac = placaFiltro;
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

	function abrirEditar(r: Renta) {
		form = {
			placa: r.placa,
			idCliente: r.idCliente,
			nombreCliente: r.nombreCliente,
			noLicencia: r.noLicencia ?? '',
			nacionalidad: r.nacionalidad ?? '',
			fechaRecogida: r.fechaRecogida,
			horaRecogida: r.horaRecogida ?? '',
			ubicacionRecogida: r.ubicacionRecogida ?? '',
			fechaRetorno: r.fechaRetorno,
			horaRetorno: r.horaRetorno ?? '',
			ubicacionRetorno: r.ubicacionRetorno ?? '',
			diasCalculados: r.diasCalculados,
			horasExtras: r.horasExtras,
			valorDia: r.valorDia,
			valorHoraExtra: r.valorHoraExtra,
			valorDiaExtra: r.valorDiaExtra,
			costoLavado: r.costoLavado,
			costoSilla: r.costoSilla,
			costoRetorno: r.costoRetorno,
			costoDomicilio: r.costoDomicilio,
			costoCables: r.costoCables,
			costoInversor: r.costoInversor,
			descuento: r.descuento,
			abono: r.abono,
			observaciones: r.observaciones ?? '',
			kmSalida: r.kmSalida,
			tanqueSalida: r.tanqueSalida ?? 'Lleno',
			idReserva: r.idReserva
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
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				await rentaApi.actualizar(sid(), editandoId, form);
				toast.success(`Renta #${editandoId} actualizada.`);
			} else {
				const creada = await rentaApi.crear(sid(), form);
				toast.success(`Renta #${creada.id} creada.`);
			}
			modalOpen = false;
			await cargar();
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar la renta.';
		} finally {
			guardando = false;
		}
	}

	// ── Cierre ──
	function abrirCierre(r: Renta) {
		cerrandoId = r.id;
		cierre = defaultCierre();
		cierre.kmFinal = r.kmSalida;
		cierreError = '';
	}

	async function confirmarCierre() {
		if (cerrandoId === null) return;
		cierreError = '';
		cerrando = true;
		try {
			const cerrada = await rentaApi.cerrar(sid(), cerrandoId, cierre);
			toast.success(`Renta #${cerrandoId} cerrada (saldo ${formatCOP(cerrada.saldoPendiente)}).`);
			cerrandoId = null;
			await cargar();
		} catch (e) {
			cierreError = e instanceof ApiError ? e.message : 'No se pudo cerrar la renta.';
		} finally {
			cerrando = false;
		}
	}

	// ── Pago ──
	function abrirPago(r: Renta) {
		pagandoId = r.id;
		pago = defaultPago();
		pagoError = '';
	}

	async function confirmarPago() {
		if (pagandoId === null) return;
		pagoError = '';
		guardandoPago = true;
		try {
			await rentaApi.registrarPago(sid(), pagandoId, pago);
			toast.success('Pago registrado.');
			pagandoId = null;
			await cargar();
		} catch (e) {
			pagoError = e instanceof ApiError ? e.message : 'No se pudo registrar el pago.';
		} finally {
			guardandoPago = false;
		}
	}

	// ── Inspección ──
	function abrirInspeccion(r: Renta, tipo: 'Salida' | 'Entrada') {
		inspeccionandoId = r.id;
		inspeccionTipo = tipo;
		inspeccion = defaultInspeccion(tipo);
		if (!inspeccion.kilometraje && tipo === 'Salida') inspeccion.kilometraje = r.kmSalida;
		inspeccionError = '';
	}

	async function confirmarInspeccion() {
		if (inspeccionandoId === null) return;
		inspeccionError = '';
		guardandoInspeccion = true;
		try {
			await rentaApi.registrarInspeccion(sid(), inspeccionandoId, inspeccion);
			toast.success(`Inspección de ${inspeccionTipo} registrada.`);
			inspeccionandoId = null;
			await cargar();
		} catch (e) {
			inspeccionError = e instanceof ApiError ? e.message : 'No se pudo registrar la inspección.';
		} finally {
			guardandoInspeccion = false;
		}
	}

	async function confirmarCancelar() {
		if (cancelarId === null) return;
		cancelando = true;
		try {
			const r = await rentaApi.cancelar(sid(), cancelarId);
			toast.success(
				r.cancelada ? `Renta #${cancelarId} cancelada.` : `La renta #${cancelarId} ya estaba cancelada.`
			);
			cancelarId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo cancelar la renta.');
			cancelarId = null;
		} finally {
			cancelando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await rentaApi.eliminar(sid(), eliminarId);
			toast.success('Renta eliminada.');
			eliminarId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar la renta.');
			eliminarId = null;
		} finally {
			eliminando = false;
		}
	}

	// ── Impresión ──
	async function abrirImprimir(r: Renta) {
		// El listado no incluye pagos/inspecciones; obtener el detalle completo
		try {
			imprimirRenta = await rentaApi.obtener(sid(), r.id);
		} catch {
			imprimirRenta = r;
		}
	}

	function imprimir() {
		imprimirDocumento();
	}

	function cerrarImpresion() {
		imprimirRenta = null;
		if (typeof document !== 'undefined') {
			document.body.classList.remove('printing');
		}
	}

	// ── Contrato (documento separado de la orden, papel Carta) ──
	function abrirContrato() {
		imprimirContrato = imprimirRenta;
		imprimirRenta = null;
	}

	function cerrarContrato() {
		imprimirContrato = null;
		if (typeof document !== 'undefined') {
			document.body.classList.remove('printing');
		}
	}

	// ── Presentación ──
	function estadoClases(estado: string): string {
		if (estado === 'Activo' || estado === 'Activa') return 'bg-primary/10 text-primary border-primary/25';
		if (estado === 'Cerrada') return 'bg-estado-activo/10 text-estado-activo border-estado-activo/25';
		if (estado === 'Cancelada') return 'bg-peligro/10 text-peligro border-peligro/25';
		return 'bg-text-secondary/10 text-text-secondary border-text-secondary/25';
	}

	function fmtHora(h: string | null): string {
		if (!h) return '—';
		const [hh, mm] = h.split(':');
		return `${hh}:${mm}`;
	}

	const rentaActiva = (r: Renta) => r.estado === 'Activo' || r.estado === 'Activa';

	const tablaRentas = $derived(rentas as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'contrato', header: 'Contrato' },
		{ key: 'cliente', header: 'Cliente' },
		{ key: 'vehiculo', header: 'Vehículo' },
		{ key: 'itinerario', header: 'Itinerario' },
		{ key: 'financiero', header: 'Total / Saldo' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Rentas — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Rentas</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{rentas.length} renta{rentas.length === 1 ? '' : 's'} · cierre con devolución real, pagos e inspecciones
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Nueva Renta
		</button>
	</div>

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por cliente, placa o estado..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={estadoFiltro} aria-label="Filtrar por estado">
			<option value="">Todos los estados</option>
			{#each ['Activo', 'Cerrada', 'Cancelada'] as est}
				<option value={est}>{est}</option>
			{/each}
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
				<p class="text-sm text-text-secondary">Cargando rentas...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaRentas}
			emptyTitle="No hay rentas"
			emptyDescription="Crea la primera renta con el botón «Nueva Renta»."
			emptyIcon="clipboard"
		>
			{#snippet children(col, item)}
				{@const r = item as unknown as Renta}
				{#if col.key === 'contrato'}
					<div class="whitespace-nowrap">
						<p class="font-bold text-primary tabular-nums" title="Número de contrato">{formatContrato(r.anioContrato, r.noContrato)}</p>
						<p class="text-[10px] text-text-secondary tabular-nums">Id {r.id}</p>
					</div>
				{:else if col.key === 'cliente'}
					<div class="max-w-[200px]">
						<p class="font-semibold text-text-primary truncate">{r.nombreCliente}</p>
						{#if r.nacionalidad}
							<p class="text-xs text-text-secondary truncate">{r.nacionalidad}</p>
						{/if}
					</div>
				{:else if col.key === 'vehiculo'}
					<div>
						<p class="text-text-primary truncate max-w-[160px]">{r.vehiculo || '—'}</p>
						<p class="text-xs text-text-secondary font-mono">{r.placa || 'Sin placa'}</p>
					</div>
				{:else if col.key === 'itinerario'}
					<div class="whitespace-nowrap">
						<p class="text-text-primary tabular-nums text-xs">{formatDate(r.fechaRecogida)} <span class="text-text-secondary">{fmtHora(r.horaRecogida)}</span></p>
						<p class="text-xs text-text-secondary tabular-nums">→ {formatDate(r.fechaRetorno)} <span class="text-text-secondary">{fmtHora(r.horaRetorno)}</span></p>
						<p class="text-xs text-text-secondary">
							{r.diasCalculados} día{r.diasCalculados === 1 ? '' : 's'}{r.horasExtras > 0 ? ` + ${r.horasExtras}h` : ''}
						</p>
					</div>
				{:else if col.key === 'financiero'}
					<div class="text-right whitespace-nowrap">
						<p class="font-bold text-text-primary tabular-nums">{formatCOP(r.total)}</p>
						<p class="text-xs text-text-secondary tabular-nums">Saldo: <span class="font-semibold {parseFloat(r.saldoPendiente) > 0 ? 'text-alerta' : 'text-exito'}">{formatCOP(r.saldoPendiente)}</span></p>
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
							title="Imprimir orden de renta"
							onclick={() => abrirImprimir(r)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
						</button>
						{#if rentaActiva(r)}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
								title="Registrar pago"
								onclick={() => abrirPago(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z" /></svg>
							</button>
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
								title="Cerrar renta (devolución)"
								onclick={() => abrirCierre(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
							</button>
						{/if}
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Registrar inspección"
							onclick={() => abrirInspeccion(r, 'Salida')}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" /><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(r)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						{#if rentaActiva(r)}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-alerta hover:bg-alerta/10 transition-colors"
								title="Cancelar renta"
								onclick={() => {
									cancelarId = r.id;
									cancelarNombre = r.nombreCliente;
								}}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9.75 9.75l4.5 4.5m0-4.5l-4.5 4.5M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
							</button>
						{/if}
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
							title="Eliminar"
							onclick={() => (eliminarId = r.id)}
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
	title={editando ? `Editar renta #${editandoId}` : 'Nueva renta'}
	subtitle={editando ? 'Modifica los datos y guarda los cambios.' : 'Registra una renta para un cliente.'}
	onClose={() => (modalOpen = false)}
	width="max-w-3xl"
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<!-- Cliente -->
			<div class="col-span-full mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">1</span>
					Cliente
				</h3>
			</div>
			<FormField label="Cliente registrado" hint="Opcional: se autocompleta el nombre.">
				<select class="input" onchange={onClienteChange}>
					<option value="">— Sin cliente registrado —</option>
					{#each clientes as c}
						<option value={c.cliente.id} selected={form.idCliente === c.cliente.id}>{c.cliente.nombreCompleto}</option>
					{/each}
				</select>
				<button
					type="button"
					class="mt-2 inline-flex items-center gap-1.5 text-xs font-semibold text-primary hover:text-primary-focus transition-colors"
					onclick={() => (clienteModalOpen = true)}
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
					¿No está registrado? Crear nuevo cliente
				</button>
			</FormField>
			<FormField label="Nombre del cliente" required>
				<input class="input" placeholder="Nombre para la renta" bind:value={form.nombreCliente} maxlength="200" />
			</FormField>
			<FormField label="Nacionalidad">
				<input class="input" placeholder="Ej: Colombiana" bind:value={form.nacionalidad} maxlength="80" />
			</FormField>
			<FormField label="No. licencia de conducción">
				<input class="input" placeholder="Ej: LC-102345678" bind:value={form.noLicencia} maxlength="50" />
			</FormField>

			<!-- Vehículo -->
			<div class="col-span-full mt-4 mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">2</span>
					Vehículo
				</h3>
			</div>
			<FormField label="Placa" required hint="Autocompleta el kilometraje actual.">
				<select class="input" onchange={onPlacaChange}>
					<option value="">— Seleccionar vehículo —</option>
					{#each autos as a}
						<option value={a.placa} selected={form.placa === a.placa}>{a.placa} · {a.marca} {a.modelo}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Km de salida">
				<input class="input" inputmode="numeric" placeholder="Ej: 42000" bind:value={form.kmSalida} />
			</FormField>
			<FormField label="Tanque de salida">
				<select class="input" bind:value={form.tanqueSalida}>
					{#each (lists?.nivelTanque ?? ['Lleno', '3/4', '1/2', '1/4', 'Vacío']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>

			<!-- Itinerario -->
			<div class="col-span-full mt-4 mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">3</span>
					Itinerario
				</h3>
			</div>
			<FormField label="Fecha de recogida" required>
				<input class="input" type="date" bind:value={form.fechaRecogida} onchange={recalcularDias} />
			</FormField>
			<FormField label="Hora de recogida">
				<input class="input" type="time" bind:value={form.horaRecogida} />
			</FormField>
			<FormField label="Lugar de recogida">
				<input class="input" placeholder="Ej: Aeropuerto, oficina..." bind:value={form.ubicacionRecogida} maxlength="200" />
			</FormField>
			<FormField label="Fecha de retorno" required>
				<input class="input" type="date" bind:value={form.fechaRetorno} onchange={recalcularDias} />
			</FormField>
			<FormField label="Hora de retorno">
				<input class="input" type="time" bind:value={form.horaRetorno} />
			</FormField>
			<FormField label="Lugar de retorno">
				<input class="input" placeholder="Ej: Aeropuerto, oficina..." bind:value={form.ubicacionRetorno} maxlength="200" />
			</FormField>

			<!-- Tarifas -->
			<div class="col-span-full mt-4 mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">4</span>
					Tarifas y costos
				</h3>
			</div>
			<FormField label="Valor por día (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 150000" bind:value={form.valorDia} />
			</FormField>
			<FormField label="Valor hora extra (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 10000" bind:value={form.valorHoraExtra} />
			</FormField>
			<FormField label="Días calculados" hint="Se calcula de las fechas; ajustable.">
				<input class="input" type="number" min="0" step="1" bind:value={form.diasCalculados} />
			</FormField>
			<FormField label="Horas extras">
				<input class="input" type="number" min="0" step="1" bind:value={form.horasExtras} />
			</FormField>
			<FormField label="Valor día extra (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 50000" bind:value={form.valorDiaExtra} />
			</FormField>
			<FormField label="Costo lavado (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 25000" bind:value={form.costoLavado} />
			</FormField>
			<FormField label="Costo silla de bebé (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 15000" bind:value={form.costoSilla} />
			</FormField>
			<FormField label="Costo recogida/retorno (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 30000" bind:value={form.costoRetorno} />
			</FormField>
			<FormField label="Costo domicilio (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 20000" bind:value={form.costoDomicilio} />
			</FormField>
			<FormField label="Costo cables (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 10000" bind:value={form.costoCables} />
			</FormField>
			<FormField label="Costo inversor (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 8000" bind:value={form.costoInversor} />
			</FormField>
			<FormField label="Descuento (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 5000" bind:value={form.descuento} />
			</FormField>
			<FormField label="Abono inicial (COP)">
				<input class="input" inputmode="decimal" placeholder="Ej: 100000" bind:value={form.abono} />
			</FormField>

			<!-- Resumen en vivo -->
			<div class="col-span-full rounded-xl border border-border bg-alt-row/60 px-4 py-3 mt-1">
				<div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
					<div>
						<p class="text-[11px] uppercase tracking-wide text-text-secondary font-semibold">Subtotal</p>
						<p class="font-bold text-text-primary tabular-nums">{formatCOP(subtotalCalc)}</p>
					</div>
					<div>
						<p class="text-[11px] uppercase tracking-wide text-text-secondary font-semibold">Total estimado</p>
						<p class="font-black text-primary tabular-nums text-base">{formatCOP(totalCalc)}</p>
					</div>
					<div>
						<p class="text-[11px] uppercase tracking-wide text-text-secondary font-semibold">Abono</p>
						<p class="font-semibold text-text-primary tabular-nums">{formatCOP(form.abono)}</p>
					</div>
					<div>
						<p class="text-[11px] uppercase tracking-wide text-text-secondary font-semibold">Saldo pendiente</p>
						<p class="font-bold text-exito tabular-nums">{formatCOP(saldoCalc)}</p>
					</div>
				</div>
				<p class="text-[10px] text-text-secondary mt-2">
					El total final incluye impuestos (config.ini) y lo recalcula el sistema al guardar.
				</p>
			</div>

			<FormField label="Observaciones" hint="Aparecen en el documento imprimible.">
				<textarea class="input min-h-[70px] resize-y" bind:value={form.observaciones} maxlength="2000"></textarea>
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
				{editando ? 'Guardar cambios' : 'Crear renta'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal cierre -->
<Modal
	open={cerrandoId !== null}
	title={cerrandoId !== null ? `Cerrar renta #${cerrandoId}` : ''}
	subtitle="Registra la devolución real; el sistema recalcula los totales."
	onClose={() => (cerrandoId = null)}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if cierreError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{cierreError}</div>
		{/if}
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Fecha de devolución real" required>
				<input class="input" type="date" bind:value={cierre.fechaDevolucionReal} />
			</FormField>
			<FormField label="Hora de devolución">
				<input class="input" type="time" bind:value={cierre.horaDevolucionReal} />
			</FormField>
			<FormField label="Km final">
				<input class="input" inputmode="numeric" placeholder="Km al devolver" bind:value={cierre.kmFinal} />
			</FormField>
			<FormField label="Tanque final">
				<select class="input" bind:value={cierre.tanqueFinal}>
					{#each (lists?.nivelTanque ?? ['Lleno', '3/4', '1/2', '1/4', 'Vacío']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Días cobrados" hint="Opcional: ajusta los días reales.">
				<input class="input" type="number" min="0" step="1" placeholder="Mantener" bind:value={cierre.diasCalculados} />
			</FormField>
			<FormField label="Horas extras finales">
				<input class="input" type="number" min="0" step="1" placeholder="Mantener" bind:value={cierre.horasExtras} />
			</FormField>
			<FormField label="Valor día final (COP)">
				<input class="input" inputmode="decimal" placeholder="Mantener" bind:value={cierre.valorDia} />
			</FormField>
			<FormField label="Descuento final (COP)">
				<input class="input" inputmode="decimal" placeholder="Mantener" bind:value={cierre.descuento} />
			</FormField>
			<FormField label="Observaciones de la devolución">
				<textarea class="input min-h-[70px] resize-y" bind:value={cierre.observaciones} maxlength="2000"></textarea>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (cerrandoId = null)} disabled={cerrando}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarCierre} disabled={cerrando}>
			{#if cerrando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Cerrando...
			{:else}
				Cerrar renta
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal pago -->
<Modal
	open={pagandoId !== null}
	title={pagandoId !== null ? `Registrar pago — renta #${pagandoId}` : ''}
	subtitle="El abono y el saldo pendiente se actualizan automáticamente."
	onClose={() => (pagandoId = null)}
	width="max-w-md"
>
	{#snippet children()}
		{#if pagoError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{pagoError}</div>
		{/if}
		<div class="space-y-4">
			<FormField label="Monto (COP)" required>
				<input class="input" inputmode="decimal" placeholder="Ej: 200000" bind:value={pago.monto} />
			</FormField>
			<FormField label="Método de pago" required>
				<select class="input" bind:value={pago.metodoPago}>
					{#each ['Efectivo', 'Tarjeta débito', 'Tarjeta crédito', 'Transferencia', 'Nequi', 'Daviplata', 'Otro'] as m}
						<option value={m}>{m}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Concepto" required>
				<input class="input" placeholder="Ej: Abono renta" bind:value={pago.concepto} maxlength="80" />
			</FormField>
			<FormField label="Observaciones">
				<textarea class="input min-h-[60px] resize-y" bind:value={pago.observaciones} maxlength="2000"></textarea>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (pagandoId = null)} disabled={guardandoPago}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarPago} disabled={guardandoPago}>
			{#if guardandoPago}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				Registrar pago
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal inspección -->
<Modal
	open={inspeccionandoId !== null}
	title={inspeccionandoId !== null ? `Inspección de ${inspeccionTipo} — renta #${inspeccionandoId}` : ''}
	subtitle="Verificación del estado del vehículo al entregar o recibir."
	onClose={() => (inspeccionandoId = null)}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if inspeccionError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{inspeccionError}</div>
		{/if}
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<div class="col-span-full mb-1">
				<div class="inline-flex rounded-lg border border-border p-1 bg-alt-row/60" role="tablist" aria-label="Tipo de inspección">
					{#each ['Salida', 'Entrada'] as t}
						<button
							type="button"
							class="px-3 py-1.5 rounded-md text-sm font-semibold transition-colors {inspeccionTipo === t ? 'bg-primary text-white shadow' : 'text-text-secondary hover:text-text-primary'}"
							role="tab"
							aria-selected={inspeccionTipo === t}
							onclick={() => {
								inspeccionTipo = t as 'Salida' | 'Entrada';
								inspeccion = defaultInspeccion(inspeccionTipo);
								if (inspeccionTipo === 'Salida' && pagandoId === null && inspeccionandoId !== null) {
									const actual = rentas.find((r) => r.id === inspeccionandoId);
									if (actual) inspeccion.kilometraje = actual.kmSalida;
								}
							}}
						>
							{t}
						</button>
					{/each}
				</div>
			</div>
			<FormField label="Kilometraje" required>
				<input class="input" inputmode="numeric" placeholder="Km actual" bind:value={inspeccion.kilometraje} />
			</FormField>
			<FormField label="Nivel de gasolina" required>
				<select class="input" bind:value={inspeccion.nivelGasolina}>
					{#each (lists?.nivelTanque ?? ['Lleno', '3/4', '1/2', '1/4', 'Vacío']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Limpieza">
				<select class="input" bind:value={inspeccion.limpieza}>
					{#each ['Limpio', 'Aceptable', 'Sucio'] as l}
						<option value={l}>{l}</option>
					{/each}
				</select>
			</FormField>
			<div class="col-span-full grid grid-cols-2 sm:grid-cols-4 gap-2">
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors">
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneRepuesto} />
					Llanta repuesto
				</label>
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors">
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneGatoCruceta} />
					Gato / cruceta
				</label>
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors">
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneKitCarretera} />
					Kit carretera
				</label>
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors">
					<input type="checkbox" class="accent-primary" bind:checked={inspeccion.tieneDocumentos} />
					Documentos
				</label>
			</div>
			<FormField label="Daños de carrocería">
				<textarea class="input min-h-[60px] resize-y" placeholder="Describir golpes, rayones..." bind:value={inspeccion.danosCarroceria} maxlength="2000"></textarea>
			</FormField>
			<FormField label="Observaciones">
				<textarea class="input min-h-[60px] resize-y" bind:value={inspeccion.observaciones} maxlength="2000"></textarea>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (inspeccionandoId = null)} disabled={guardandoInspeccion}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarInspeccion} disabled={guardandoInspeccion}>
			{#if guardandoInspeccion}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				Registrar inspección
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal orden imprimible -->
<Modal
	open={imprimirRenta !== null}
	title={imprimirRenta ? `Orden de renta #${String(imprimirRenta.id).padStart(4, '0')}` : ''}
	subtitle="Vista previa del documento. Al imprimir solo se muestra la orden."
	onClose={cerrarImpresion}
	width="max-w-3xl"
>
	{#snippet children()}
		{#if imprimirRenta}
			<AvisoImpresion />
			<OrdenRenta 
				renta={imprimirRenta} 
				cliente={clientes.find(c => c.cliente.id === imprimirRenta?.idCliente)?.cliente}
				auto={autos.find(a => a.placa === imprimirRenta?.placa)}
			/>
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost print-hidden" onclick={cerrarImpresion}>Cerrar</button>
		<button class="btn-outline print-hidden" onclick={abrirContrato}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" /></svg>
			Ver contrato (Carta)
		</button>
		<button class="btn-primary print-hidden" onclick={imprimir}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
			Imprimir orden
		</button>
	{/snippet}
</Modal>

<!-- Modal contrato imprimible (documento independiente, papel Carta) -->
<Modal
	open={imprimirContrato !== null}
	title={imprimirContrato ? `Contrato de renta #${String(imprimirContrato.id).padStart(4, '0')}` : ''}
	subtitle="Documento legal independiente. Se imprime en papel Carta."
	onClose={cerrarContrato}
	width="max-w-3xl"
>
	{#snippet children()}
		{#if imprimirContrato}
			<AvisoImpresion />
			<ContratoRenta
				renta={imprimirContrato}
				cliente={clientes.find((c) => c.cliente.id === imprimirContrato?.idCliente)?.cliente}
				auto={autos.find((a) => a.placa === imprimirContrato?.placa)}
			/>
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost print-hidden" onclick={cerrarContrato}>Cerrar</button>
		<button class="btn-primary print-hidden" onclick={imprimir}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
			Imprimir contrato
		</button>
	{/snippet}
</Modal>

<!-- Modal cliente embebido: crear un cliente nuevo sin salir del formulario de renta -->
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
	title="Cancelar renta"
	message={`¿Seguro que deseas cancelar la renta de ${cancelarNombre}? Los pagos e inspecciones se conservan.`}
	confirmLabel="Cancelar renta"
	loading={cancelando}
	onConfirm={confirmarCancelar}
	onCancel={() => (cancelarId = null)}
/>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar renta"
	message="¿Seguro que deseas eliminar esta renta? Los pagos e inspecciones asociados se eliminarán. Esta acción no se puede deshacer."
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>
