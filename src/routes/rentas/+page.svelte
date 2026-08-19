<script lang="ts">
	import { onMount } from 'svelte';
	import {
		rentaApi,
		reservaApi,
		clienteApi,
		autoApi,
		ApiError,
		type Renta,
		type RentaDatos,
		type RentaCierreDatos,
		type RentaCierreEditDatos,
		type ExtensionDatos,
		type ExtensionRenta,
		type PagoDatos,
		type InspeccionDatos,
		type ClienteConPii,
		type Auto,
		type BusinessLists,
		type Reserva
	} from '$lib/api';
	import { sid, session } from '$lib/stores/session.svelte';
	import { businessLists } from '$lib/stores/business.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';
	import { calcularDiasHoras } from '$lib/utils/calcularDiasHoras';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import SearchSelect, { type SearchSelectOpcion } from '$lib/components/SearchSelect.svelte';
	import ClienteFormModal from '$lib/components/ClienteFormModal.svelte';
	import OrdenRenta from '$lib/components/reports/OrdenRenta.svelte';
	import ContratoRenta from '$lib/components/reports/ContratoRenta.svelte';
	import AvisoImpresion from '$lib/components/AvisoImpresion.svelte';
	import { imprimirDocumento } from '$lib/utils/imprimir';
	import { useDebouncedEffect } from '$lib/utils/debounce.svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';

	// sid() viene del store (reemplaza el patrón `const sid = () => session.token ?? ''`
	// repetido en 15 rutas). Ver TAREA E3 del Grupo E.
	// La importación explícita reemplaza a `const sid = () => session.token ?? '';`.

	let rentas = $state<Renta[]>([]);
	let clientes = $state<ClienteConPii[]>([]);
	let autos = $state<Auto[]>([]);

	// Opciones para los combos con búsqueda: el cliente filtra por nombre y
	// por número de documento; el auto por placa, marca, modelo, tipo o color.
	const opcionesClientes = $derived<SearchSelectOpcion[]>(
		clientes.map((c) => ({
			value: String(c.cliente.id),
			label: c.cliente.nombreCompleto,
			sub: [c.cliente.tipoDoc ?? '', c.cliente.noDoc ?? ''].filter(Boolean).join(' ').trim()
		}))
	);
	const opcionesAutos = $derived<SearchSelectOpcion[]>(
		autos.map((a) => ({
			value: a.placa,
			label: `${a.placa} · ${a.marca} ${a.modelo}`,
			sub: [a.tipo ?? '', a.color ?? ''].filter(Boolean).join(' ').trim()
		}))
	);
	// TAREA 3.2 (Bloque 3 — Performance): `lists` se sirve desde el store
	// global `businessLists` (cache TTL 5 min, invalidable). La primera
	// ruta que monta dispara la carga; las siguientes leen del cache sin
	// round-trip.
	const lists = $derived<BusinessLists | null>(businessLists.lists);
	let loading = $state(true);

	// ¿El rol actual puede eliminar registros? (roles_con_eliminar de config.ini)
	const puedeEliminar = $derived(
		(lists?.rolesConEliminar ?? ['Administrador', 'Supervisor']).includes(session.user?.rol ?? '')
	);

	// Filtros
	let busqueda = $state('');
	let estadoFiltro = $state('');
	let placaFiltro = $state('');

	// Modal crear/editar
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<RentaDatos>(defaultForm());
	let formError = $state('');
	// Costos adicionales colapsables (mejora UX: 7 campos opcionales ocultos por defecto)
	let costosOpen = $state(false);

	// Modal cliente embebido (crear cliente sin salir de la renta)
	let clienteModalOpen = $state(false);

	// Modal cierre
	let cerrandoId = $state<number | null>(null);
	let cierre = $state<RentaCierreDatos>(defaultCierre());
	let cerrando = $state(false);
	let cierreError = $state('');
	let cerrarRenta = $state<Renta | null>(null);

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

	// Modal extender renta
	let extenderId = $state<number | null>(null);
	let extenderRenta = $state<Renta | null>(null);
	let extension = $state<ExtensionDatos>(defaultExtension());
	let extenderando = $state(false);
	let extenderError = $state('');
	let historialExtensiones = $state<ExtensionRenta[]>([]);
	let cargandoHistorial = $state(false);

	// Modal imprimir
	let imprimirRenta = $state<Renta | null>(null);
	// Modal contrato (documento independiente, papel Carta)
	let imprimirContrato = $state<Renta | null>(null);

	// Modal editar renta cerrada (solo Administrador)
	let editandoCerradaId = $state<number | null>(null);
	let editandoCerradaRenta = $state<Renta | null>(null);
	let editCerrada = $state<RentaCierreEditDatos>(defaultEditCerrada());
	let editandoCerrada = $state(false);
	let editCerradaError = $state('');

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
			valorGasolina: '0',
			descuento: '0',
			cobraIva: false,
			tieneComision: false,
			comision: '0',
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

	function defaultExtension(): ExtensionDatos {
		return {
			tipo: 'horas',
			cantidad: 1,
			valor: '',
			observaciones: ''
		};
	}

	function defaultEditCerrada(): RentaCierreEditDatos {
		return {
			valorDia: '',
			valorHoraExtra: '',
			diasCalculados: null,
			horasExtras: null,
			descuento: '',
			observaciones: ''
		};
	}

	// ── Calculadora en vivo (espejo del cálculo del backend) ──
	// El IVA solo se aplica si el checkbox «cobrar IVA» está activo.
	const brutoCalc = $derived(
		(parseFloat(form.valorDia) || 0) * form.diasCalculados +
			(parseFloat(form.valorHoraExtra) || 0) * form.horasExtras
	);
	const extrasCalc = $derived(
		['valorDiaExtra', 'costoLavado', 'costoSilla', 'costoRetorno', 'costoDomicilio', 'costoCables', 'costoInversor', 'valorGasolina']
			.reduce((acc, k) => acc + (parseFloat(form[k as keyof RentaDatos] as string) || 0), 0)
	);
	const subtotalCalc = $derived(Math.max(0, brutoCalc + extrasCalc - (parseFloat(form.descuento) || 0)));
	const tasaIva = $derived(lists?.impuestoPorcentaje ?? 19);
	const ivaCalc = $derived(
		form.cobraIva ? Math.round(subtotalCalc * (tasaIva / 100) * 100) / 100 : 0
	);
	const totalCalc = $derived(subtotalCalc + ivaCalc);
	// Comisión: solo aplica si el checkbox está activo; neto = total − comisión
	const comisionCalc = $derived(form.tieneComision ? Math.max(0, parseFloat(form.comision) || 0) : 0);
	const netoCalc = $derived(Math.max(0, totalCalc - comisionCalc));
	const saldoCalc = $derived(Math.max(0, totalCalc - (parseFloat(form.abono) || 0)));

	function recalcularDias() {
		// Regla unificada (espejo del cierre): cada 24 h = 1 día; excedente ≤ 3 h
		// → horas extras (redondeadas hacia arriba); excedente > 3 h → día completo.
		// Sin horas → diferencia de días calendario (comportamiento histórico).
		const { dias, horas } = calcularDiasHoras(
			form.fechaRecogida,
			form.horaRecogida ?? '',
			form.fechaRetorno,
			form.horaRetorno ?? ''
		);
		form.diasCalculados = dias;
		form.horasExtras = horas;
	}

	// ── Auto-cálculo de días/horas en el cierre (espejo del backend) ──
	// Regla unificada en calcularDiasHoras: cada 24 h = 1 día; el excedente de
	// hasta 3 h se cobra como horas extras (redondeadas hacia arriba); si el
	// excedente supera 3 h se cobra el día completo.
	function calcularCierre() {
		const r = cerrarRenta;
		if (!r) return;
		// Solo se auto-calculan días/horas cuando hay hora de devolución real
		// (sin ella no se puede aplicar la regla; los campos quedan «Mantener»
		// y el backend conserva el valor original de la renta).
		if (!cierre.horaDevolucionReal || !r.horaRecogida) return;
		const { dias, horas } = calcularDiasHoras(
			r.fechaRecogida,
			r.horaRecogida ?? '',
			cierre.fechaDevolucionReal,
			cierre.horaDevolucionReal ?? ''
		);
		cierre.diasCalculados = dias;
		cierre.horasExtras = horas;
	}

	function onClienteChange(v: string) {
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

	function onPlacaChange(v: string) {
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
		// TAREA 3.2 + 3.3 (Bloque 3 — Performance):
		//  - `businessLists.ensure` carga las listas si no están en cache
		//    (TTL 5 min) o reutiliza las cacheadas — evita 1 round-trip por
		//    cada navegación a /rentas.
		//  - `Promise.all` paraleliza las cargas independientes (clientes,
		//    autos, listas) que antes corrían en secuencia → 3 round-trips
		//    en paralelo en vez de 3 secuenciales.
		await Promise.all([
			businessLists.ensure(sid()).catch(() => null),
			clienteApi.listar(sid()).then((c) => (clientes = c)).catch(() => (clientes = [])),
			autoApi.listar(sid()).then((a) => (autos = a)).catch(() => (autos = []))
		]);
		await cargar();

		// Precarga desde una reserva (?desdeReserva=<id>): abre el formulario
		// con los datos de la reserva y limpia el query param para que un
		// refresh no re-dispare la precarga.
		const idReserva = Number(page.url.searchParams.get('desdeReserva'));
		if (idReserva > 0) {
			try {
				const r = await reservaApi.obtener(sid(), idReserva);
				precargarDesdeReserva(r);
			} catch (e) {
				toast.error(
					e instanceof ApiError
						? e.message
						: 'No se pudo cargar la reserva para crear la renta.'
				);
			} finally {
				goto('/rentas', { replaceState: true });
			}
		}
	});

	// Recarga con debounce al cambiar filtros. `skipFirst: true` porque la
	// carga inicial la hace onMount; `immediateIf` recarga sin debounce al
	// vaciar la búsqueda (mismo comportamiento que el patrón manual previo).
	const scheduleReload = useDebouncedEffect(cargar, {
		skipFirst: true,
		immediateIf: () => !busqueda.trim()
	});
	$effect(() => {
		const _b = busqueda;
		const _e = estadoFiltro;
		const _p = placaFiltro;
		scheduleReload();
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
			valorGasolina: r.valorGasolina ?? '0',
			descuento: r.descuento,
			cobraIva: r.cobraIva,
			tieneComision: r.tieneComision,
			comision: r.comision,
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

	/// Abre el formulario de NUEVA renta con los datos de una reserva
	/// (cliente, vehículo, fechas, tarifas y abono) y enlaza idReserva para
	/// conservar la trazabilidad. El km de salida se autocompleta del auto.
	function precargarDesdeReserva(r: Reserva) {
		const auto = autos.find((a) => a.placa === r.placaAsignada);
		form = {
			...defaultForm(),
			placa: r.placaAsignada,
			idCliente: r.idCliente,
			nombreCliente: r.nombreCliente,
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
			valorHoraExtra: r.valorHoraAdic,
			abono: r.abono,
			observaciones: r.observaciones ?? '',
			kmSalida: auto ? String(auto.kilometraje ?? '') : '',
			tanqueSalida: 'Lleno',
			idReserva: r.id
		};
		editando = false;
		editandoId = null;
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
		cerrarRenta = r;
		cierre = defaultCierre();
		cierre.kmFinal = r.kmSalida;
		cierreError = '';
		calcularCierre();
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

	// ── Cambiar vehículo (sin cerrar la renta) ──
	let cambiarAutoId = $state<number | null>(null);
	let cambiarAutoPlaca = $state('');
	let cambiarAutoError = $state('');
	let guardandoCambioAuto = $state(false);

	const autosParaCambio = $derived.by(() => {
		const actual = rentas.find((r) => r.id === cambiarAutoId);
		return autos.filter((a) => a.estado === 'Disponible' || a.placa === actual?.placa);
	});

	function abrirCambiarAuto(r: Renta) {
		cambiarAutoId = r.id;
		cambiarAutoPlaca = r.placa ?? '';
		cambiarAutoError = '';
	}

	async function confirmarCambiarAuto() {
		if (cambiarAutoId === null) return;
		cambiarAutoError = '';
		guardandoCambioAuto = true;
		try {
			const cambiada = await rentaApi.cambiarAuto(sid(), cambiarAutoId, cambiarAutoPlaca);
			toast.success(
				`Renta #${cambiarAutoId}: vehículo cambiado a ${cambiada.placa ?? 'sin placa'}.`
			);
			cambiarAutoId = null;
			await cargar();
		} catch (e) {
			cambiarAutoError = e instanceof ApiError ? e.message : 'No se pudo cambiar el vehículo.';
		} finally {
			guardandoCambioAuto = false;
		}
	}

	// ── Extender renta ──
	async function abrirExtender(r: Renta) {
		extenderId = r.id;
		extenderRenta = r;
		extension = defaultExtension();
		extenderError = '';
		historialExtensiones = [];
		// Cargar historial de extensiones
		cargandoHistorial = true;
		try {
			historialExtensiones = await rentaApi.listarExtensiones(sid(), r.id);
		} catch {
			// Ignorar errores al cargar historial
		} finally {
			cargandoHistorial = false;
		}
	}

	async function confirmarExtender() {
		if (extenderId === null) return;
		if (!extension.valor || parseFloat(extension.valor) <= 0) {
			extenderError = 'El valor de la extensión es obligatorio y debe ser mayor a cero.';
			return;
		}
		if (extension.cantidad <= 0) {
			extenderError = 'La cantidad debe ser mayor a cero.';
			return;
		}
		extenderando = true;
		try {
			await rentaApi.extender(sid(), extenderId, extension);
			toast.success(
				extension.tipo === 'horas'
					? `Renta extendida +${extension.cantidad}h.`
					: `Renta extendida +${extension.cantidad} día(s).`
			);
			extenderId = null;
			await cargar();
		} catch (e) {
			extenderError = e instanceof ApiError ? e.message : 'No se pudo extender la renta.';
		} finally {
			extenderando = false;
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

	// ── Editar renta cerrada (solo Administrador) ──
	function abrirEditarCerrada(r: Renta) {
		editandoCerradaId = r.id;
		editandoCerradaRenta = r;
		// Pre-cargar valores actuales para facilitar la corrección
		editCerrada = {
			valorDia: r.valorDia,
			valorHoraExtra: r.valorHoraExtra,
			diasCalculados: r.diasCalculados,
			horasExtras: r.horasExtras,
			descuento: r.descuento,
			observaciones: ''
		};
		editCerradaError = '';
	}

	async function confirmarEditarCerrada() {
		if (editandoCerradaId === null || !editCerrada.observaciones?.trim()) {
			editCerradaError = 'Debe indicar el motivo de la corrección (obligatorio para auditoría).';
			return;
		}
		editandoCerrada = true;
		try {
			await rentaApi.editarCerrada(sid(), editandoCerradaId, editCerrada);
			toast.success('Renta cerrada actualizada. Valores recalculados.');
			editandoCerradaId = null;
			await cargar();
		} catch (e) {
			editCerradaError = e instanceof ApiError ? e.message : 'No se pudo editar la renta cerrada.';
		} finally {
			editandoCerrada = false;
		}
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

	const columnas = [
		{ key: 'contrato', header: 'Contrato' },
		{ key: 'cliente', header: 'Cliente' },
		{ key: 'vehiculo', header: 'Vehículo' },
		{ key: 'itinerario', header: 'Itinerario' },
		{ key: 'financiero', header: 'Total / Saldo' },
		{ key: 'comision', header: 'Comisión' },
		{ key: 'valorNeto', header: 'Valor neto' },
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
			items={rentas}
			emptyTitle="No hay rentas"
			emptyDescription="Crea la primera renta con el botón «Nueva Renta»."
			emptyIcon="clipboard"
		>
			{#snippet children(col, item)}
				{@const r = item}
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
				{:else if col.key === 'comision'}
					<p class="text-right tabular-nums whitespace-nowrap {parseFloat(r.comision) > 0 ? 'font-semibold text-peligro' : 'text-text-secondary/50'}">
						{parseFloat(r.comision) > 0 ? `-${formatCOP(r.comision)}` : '—'}
					</p>
				{:else if col.key === 'valorNeto'}
					<p class="text-right font-semibold text-text-primary tabular-nums whitespace-nowrap">{formatCOP(r.valorNeto)}</p>
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
								title="Cambiar vehículo sin cerrar la renta"
								onclick={() => abrirCambiarAuto(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" /></svg>
							</button>
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
								title="Cerrar renta (devolución)"
								onclick={() => abrirCierre(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
							</button>
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-exito hover:bg-exito/10 transition-colors"
								title="Extender renta (agregar horas/días)"
								onclick={() => abrirExtender(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
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
						{#if r.estado === 'Cerrada' && puedeEliminar}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-alerta hover:bg-alerta/10 transition-colors"
								title="Editar renta cerrada (corregir digitación)"
								onclick={() => abrirEditarCerrada(r)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M11.42 15.17l-5.1-5.1m0 0L11.42 4.97m-5.1 5.1H21M3 3h18v18H3V3z" /></svg>
							</button>
						{/if}
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
					<span>{String((item as unknown as Record<string, unknown>)[col.key] ?? '—')}</span>
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
	width="max-w-6xl"
	fullHeight
	rawBody
>
	{#snippet children()}
		<div class="flex grow min-h-0">
			<!-- ── Panel izquierdo: campos (scrollable solo si es necesario) ── -->
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
						<input class="input" placeholder="Nombre para la renta" bind:value={form.nombreCliente} maxlength="200" />
					</FormField>
					<div class="grid grid-cols-2 gap-x-3">
						<FormField label="Nacionalidad" dense>
							<input class="input" placeholder="Ej: Colombiana" bind:value={form.nacionalidad} maxlength="80" />
						</FormField>
						<FormField label="No. licencia" dense>
							<input class="input" placeholder="LC-102345678" bind:value={form.noLicencia} maxlength="50" />
						</FormField>
					</div>
				</div>

				<!-- ── 2. Vehículo ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">2</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Vehículo</h3>
				</div>
				<div class="grid grid-cols-3 gap-x-3 mb-3">
					<SearchSelect
						label="Placa"
						required
						dense
						hint={editando ? 'Para cambiar el auto de una renta activa usa la acción «Cambiar vehículo» de la lista.' : 'Busca por placa, marca o modelo; autocompleta km'}
						value={form.placa ?? ''}
						opciones={opcionesAutos}
						onchange={onPlacaChange}
						placeholder="Buscar placa, marca o modelo…"
						vacioLabel="— Seleccionar —"
						disabled={editando}
					/>
					<FormField label="Km de salida" dense>
						<input class="input" inputmode="numeric" placeholder="Ej: 42000" bind:value={form.kmSalida} />
					</FormField>
					<FormField label="Tanque salida" dense>
						<select class="input" bind:value={form.tanqueSalida}>
							{#each (lists?.nivelTanque ?? ['Lleno', '3/4', '1/2', '1/4', 'Vacío']) as t}
								<option value={t}>{t}</option>
							{/each}
						</select>
					</FormField>
				</div>

				<!-- ── 3. Itinerario ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">3</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Itinerario</h3>
				</div>
				<div class="grid grid-cols-3 gap-x-3 mb-3">
					<FormField label="Fecha recogida" required dense>
						<input class="input" type="date" bind:value={form.fechaRecogida} onchange={recalcularDias} />
					</FormField>
					<FormField label="Hora recogida" dense>
						<input class="input" type="time" bind:value={form.horaRecogida} onchange={recalcularDias} />
					</FormField>
					<FormField label="Lugar recogida" dense>
						<input class="input" placeholder="Aeropuerto, oficina…" bind:value={form.ubicacionRecogida} maxlength="200" />
					</FormField>
					<FormField label="Fecha retorno" required dense>
						<input class="input" type="date" bind:value={form.fechaRetorno} onchange={recalcularDias} />
					</FormField>
					<FormField label="Hora retorno" dense>
						<input class="input" type="time" bind:value={form.horaRetorno} onchange={recalcularDias} />
					</FormField>
					<FormField label="Lugar retorno" dense>
						<input class="input" placeholder="Aeropuerto, oficina…" bind:value={form.ubicacionRetorno} maxlength="200" />
					</FormField>
				</div>

				<!-- ── 4. Tarifas base ── -->
				<div class="flex items-center gap-2 mb-2.5 mt-2">
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">4</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Tarifas base</h3>
				</div>
				<div class="grid grid-cols-4 gap-x-3 mb-3">
					<FormField label="Valor por día" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="150000" bind:value={form.valorDia} />
					</FormField>
					<FormField label="Días calculados" hint="Auto desde fechas" dense>
						<input class="input" type="number" min="0" step="1" bind:value={form.diasCalculados} />
					</FormField>
					<FormField label="Valor hora extra" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="10000" bind:value={form.valorHoraExtra} />
					</FormField>
					<FormField label="Horas extras" dense>
						<input class="input" type="number" min="0" step="1" bind:value={form.horasExtras} />
					</FormField>
				</div>
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors mb-3 w-fit">
					<input type="checkbox" class="accent-primary" bind:checked={form.cobraIva} />
					Cobrar IVA <span class="text-xs text-text-secondary">({tasaIva}% — solo si se marca)</span>
				</label>

				<!-- ── 5. Costos adicionales (colapsable) ── -->
				<button
					type="button"
					onclick={() => (costosOpen = !costosOpen)}
					class="w-full flex items-center gap-2 mb-2 mt-2 group text-left"
					aria-expanded={costosOpen}
				>
					<span class="w-5 h-5 rounded-md bg-primary/10 text-primary flex items-center justify-center text-[11px] font-bold">5</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z" /></svg>
					<h3 class="text-[11px] font-bold uppercase tracking-wider text-primary">Costos adicionales</h3>
					<span class="text-[10px] text-text-secondary bg-alt-row px-1.5 py-0.5 rounded">
						{costosOpen ? '8 campos' : '8 opcionales · ocultos'}
					</span>
					<svg xmlns="http://www.w3.org/2000/svg" class="ml-auto w-4 h-4 text-text-secondary group-hover:text-text-primary transition-transform {costosOpen ? 'rotate-90' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" /></svg>
				</button>
				{#if costosOpen}
					<div class="grid grid-cols-3 gap-x-3 mb-3 animate-[modal-fade-in_150ms_ease-out]">
						<FormField label="Valor día extra" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="50000" bind:value={form.valorDiaExtra} />
						</FormField>
						<FormField label="Costo lavado" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="25000" bind:value={form.costoLavado} />
						</FormField>
						<FormField label="Silla de bebé" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="15000" bind:value={form.costoSilla} />
						</FormField>
						<FormField label="Recogida/retorno" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="30000" bind:value={form.costoRetorno} />
						</FormField>
						<FormField label="Domicilio" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="20000" bind:value={form.costoDomicilio} />
						</FormField>
						<FormField label="Cables" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="10000" bind:value={form.costoCables} />
						</FormField>
						<FormField label="Inversor" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="8000" bind:value={form.costoInversor} />
						</FormField>
						<FormField label="Gasolina" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="30000" bind:value={form.valorGasolina} />
						</FormField>
					</div>
				{/if}

				<!-- ── 6. Descuento y abono ── -->
				<div class="grid grid-cols-2 gap-x-3">
					<FormField label="Descuento" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="5000" bind:value={form.descuento} />
					</FormField>
					<FormField label="Abono inicial" hint="COP" dense>
						<input class="input" inputmode="decimal" placeholder="100000" bind:value={form.abono} />
					</FormField>
				</div>
				<label class="flex items-center gap-2 text-sm text-text-primary cursor-pointer rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors mt-3 w-fit">
					<input type="checkbox" class="accent-primary" bind:checked={form.tieneComision} />
					Cobrar comisión <span class="text-xs text-text-secondary">(se resta del total → valor neto)</span>
				</label>
				{#if form.tieneComision}
					<div class="mt-2 max-w-[240px]">
						<FormField label="Valor comisión" hint="COP" dense>
							<input class="input" inputmode="decimal" placeholder="50000" bind:value={form.comision} />
						</FormField>
					</div>
				{/if}
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
							{form.cobraIva ? `IVA ${tasaIva}% incluido` : 'Sin IVA (checkbox desactivado)'}
						</p>
						{#if form.tieneComision}
							<p class="text-[10px] opacity-90 mt-0.5 font-semibold">Valor neto: {formatCOP(netoCalc)}</p>
						{/if}
					</div>
					<!-- Desglose compacto -->
					<div class="space-y-1 text-xs">
						<div class="flex justify-between">
							<span class="text-text-secondary">Subtotal</span>
							<span class="font-semibold text-text-primary tabular-nums">{formatCOP(subtotalCalc)}</span>
						</div>
						{#if form.cobraIva}
							<div class="flex justify-between">
								<span class="text-text-secondary">IVA ({tasaIva}%)</span>
								<span class="font-semibold text-text-primary tabular-nums">{formatCOP(ivaCalc)}</span>
							</div>
						{/if}
						{#if form.tieneComision}
							<div class="flex justify-between">
								<span class="text-text-secondary">Comisión</span>
								<span class="font-semibold text-text-primary tabular-nums">-{formatCOP(comisionCalc)}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-text-secondary font-semibold">Valor neto</span>
								<span class="font-bold text-text-primary tabular-nums">{formatCOP(netoCalc)}</span>
							</div>
						{/if}
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
						placeholder="Aparecen en el documento imprimible…"
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
							{editando ? 'Guardar cambios' : 'Crear renta'}
						{/if}
					</button>
					<button class="btn-ghost w-full" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
				</div>
			</div>
		</div>
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
				<input class="input" type="date" bind:value={cierre.fechaDevolucionReal} onchange={calcularCierre} />
			</FormField>
			<FormField label="Hora de devolución" hint="Al cambiar se recalculan días/horas">
				<input class="input" type="time" bind:value={cierre.horaDevolucionReal} onchange={calcularCierre} />
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
			<FormField label="Días cobrados" hint="Auto desde la devolución real (excedente > 3 h = día completo).">
				<input class="input" type="number" min="0" step="1" placeholder="Mantener" bind:value={cierre.diasCalculados} />
			</FormField>
			<FormField label="Horas extras finales" hint="Excedente ≤ 3 h, redondeadas hacia arriba.">
				<input class="input" type="number" min="0" step="1" placeholder="Mantener" bind:value={cierre.horasExtras} />
			</FormField>
			<FormField label="Valor día final (COP)">
				<input class="input" inputmode="decimal" placeholder="Mantener" bind:value={cierre.valorDia} />
			</FormField>
			<FormField label="Valor hora extra final (COP)">
				<input class="input" inputmode="decimal" placeholder="Mantener" bind:value={cierre.valorHoraExtra} />
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

<!-- Modal cambiar vehículo (sin cerrar la renta) -->
<Modal
	open={cambiarAutoId !== null}
	title={cambiarAutoId !== null ? `Cambiar vehículo — renta #${cambiarAutoId}` : ''}
	subtitle="Libera el auto anterior y asigna uno nuevo; la renta sigue activa."
	onClose={() => (cambiarAutoId = null)}
	width="max-w-md"
>
	{#snippet children()}
		{#if cambiarAutoError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{cambiarAutoError}</div>
		{/if}
		<FormField label="Vehículo nuevo" required hint="Solo se listan autos disponibles (más el actual).">
			<select class="input" bind:value={cambiarAutoPlaca}>
				<option value="">— Seleccionar —</option>
				{#each autosParaCambio as a}
					<option value={a.placa}>{a.placa} · {a.marca} {a.modelo}{a.estado === 'Disponible' ? '' : ' (actual)'}</option>
				{/each}
			</select>
		</FormField>
		{#if autosParaCambio.length === 0}
			<p class="text-xs text-alerta">No hay autos disponibles para el cambio. Libera uno desde la sección Autos.</p>
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (cambiarAutoId = null)} disabled={guardandoCambioAuto}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarCambiarAuto} disabled={guardandoCambioAuto || !cambiarAutoPlaca}>
			{#if guardandoCambioAuto}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Cambiando...
			{:else}
				Cambiar vehículo
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

<!-- Modal editar renta cerrada (solo Administrador) -->
<Modal
	open={editandoCerradaId !== null}
	title={editandoCerradaRenta ? `Corregir renta cerrada #${String(editandoCerradaRenta.id).padStart(4, '0')}` : ''}
	subtitle="Modifica los campos financieros y recalcula los totales."
	onClose={() => (editandoCerradaId = null)}
	width="max-w-2xl"
>
	{#snippet children()}
		<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{editCerradaError}</div>

		<div class="mb-4 rounded-lg bg-alerta/10 border border-alerta/30 px-3 py-2.5 text-sm text-alerta">
			<strong>⚠️ Atención:</strong> Solo los campos financieros se modificarán. El abono, el cliente y la placa NO se pueden editar.
		</div>

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Valor día" required>
				<input class="input" type="number" step="0.01" min="0" bind:value={editCerrada.valorDia} />
			</FormField>
			<FormField label="Valor hora extra">
				<input class="input" type="number" step="0.01" min="0" bind:value={editCerrada.valorHoraExtra} />
			</FormField>
			<FormField label="Días calculados" required>
				<input class="input" type="number" min="1" bind:value={editCerrada.diasCalculados} />
			</FormField>
			<FormField label="Horas extras">
				<input class="input" type="number" min="0" bind:value={editCerrada.horasExtras} />
			</FormField>
			<FormField label="Descuento">
				<input class="input" type="number" step="0.01" min="0" bind:value={editCerrada.descuento} />
			</FormField>
			<div class="col-span-full">
				<FormField label="Motivo de la corrección" required hint="Obligatorio para auditoría">
					<textarea class="input min-h-[60px] resize-y" placeholder="Describe el error de digitación que se corrige..." bind:value={editCerrada.observaciones} maxlength="500"></textarea>
			</FormField>
		</div>

		<div class="mt-4 p-3 rounded-lg bg-alt-row/60 border border-border">
			<p class="text-sm font-semibold text-text-primary mb-2">Valores actuales de la renta:</p>
			<p class="text-sm text-text-secondary">Total: <span class="font-semibold text-text-primary">{formatCOP(editandoCerradaRenta?.total ?? '0')}</span> | Saldo: <span class="font-semibold">{formatCOP(editandoCerradaRenta?.saldoPendiente ?? '0')}</span></p>
		</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (editandoCerradaId = null)} disabled={editandoCerrada}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarEditarCerrada} disabled={editandoCerrada}>
			{#if editandoCerrada}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				Aplicar corrección
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal extender renta -->
<Modal
	open={extenderId !== null}
	title={extenderRenta ? `Extender renta #${String(extenderRenta.id).padStart(4, '0')}` : ''}
	subtitle="Agregar horas o días extras a la renta activa."
	onClose={() => (extenderId = null)}
	width="max-w-md"
>
	{#snippet children()}
		{#if extenderError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{extenderError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Tipo de extensión" required>
				<select class="input" bind:value={extension.tipo}>
					<option value="horas">Horas extra</option>
					<option value="dias">Día(s) extra</option>
				</select>
			</FormField>
			<FormField label="Cantidad" required>
				<input class="input" type="number" min="1" bind:value={extension.cantidad} />
			</FormField>
			<FormField label="Valor unitario" required hint={extension.tipo === 'horas' ? 'Valor por hora extra' : 'Valor por día extra'}>
				<input class="input" type="number" step="0.01" min="0" placeholder="$0" bind:value={extension.valor} />
			</FormField>
			<FormField label="Observaciones">
				<input class="input" placeholder="Motivo de la extensión..." bind:value={extension.observaciones} maxlength="200" />
			</FormField>
		</div>

		{#if extenderRenta}
			<div class="mt-4 p-3 rounded-lg bg-alt-row/60 border border-border">
				<p class="text-sm font-semibold text-text-primary mb-2">Resumen:</p>
				<div class="text-sm text-text-secondary space-y-1">
					<p>Retorno actual: <span class="font-semibold">{formatDate(extenderRenta.fechaRetorno)} {fmtHora(extenderRenta.horaRetorno)}</span></p>
					<p> Nuevo retorno: <span class="font-semibold text-exito">
						{extension.tipo === 'horas'
							? `${extension.cantidad} hora(s) más`
							: `${extension.cantidad} día(s) más`}
					</span></p>
					{#if extension.valor && parseFloat(extension.valor) > 0}
						<p>Valor total extensión: <span class="font-semibold text-primary">{formatCOP((parseFloat(extension.valor) * extension.cantidad).toString())}</span></p>
					{/if}
				</div>
			</div>

			{#if historialExtensiones.length > 0}
				<div class="mt-4">
					<p class="text-sm font-semibold text-text-primary mb-2">Historial de extensiones:</p>
					<div class="space-y-2">
						{#each historialExtensiones as ext}
							<div class="p-2 rounded-lg bg-alt-row/40 border border-border text-sm">
								<div class="flex justify-between items-center">
									<span class="font-semibold text-text-primary">
										{ext.tipo === 'horas' ? `+${ext.cantidad}h` : `+${ext.cantidad}d`}
								</span>
									<span class="font-semibold text-primary">{formatCOP(ext.valorTotal)}</span>
								</div>
								<div class="text-xs text-text-secondary mt-1">
									{ext.usuario ?? 'sistema'} · {ext.createdAt ? formatDate(ext.createdAt.split(' ')[0]) : '—'}
									{#if ext.observaciones}
										<span class="ml-2">· {ext.observaciones}</span>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}
			{#if cargandoHistorial}
				<p class="text-xs text-text-secondary mt-2">Cargando historial...</p>
			{/if}
		{/if}
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (extenderId = null)} disabled={extenderando}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarExtender} disabled={extenderando}>
			{#if extenderando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Extendiendo...
			{:else}
				Aplicar extensión
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
			<OrdenRenta renta={imprimirRenta} />
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
