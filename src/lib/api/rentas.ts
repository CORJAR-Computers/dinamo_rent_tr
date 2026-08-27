// rentas.ts — Comandos y tipos para gestión de rentas, pagos, inspecciones y extensiones
import { invokeCmd } from './base';

/** Pago registrado contra una renta (repositories/renta.rs) */
export interface Pago {
	id: number;
	idRenta: number;
	fecha: string;
	monto: string;
	metodoPago: string;
	concepto: string;
	observaciones: string | null;
	usuario: string | null;
}

/** Datos de entrada para registrar un pago */
export interface PagoDatos {
	monto: string;
	metodoPago: string;
	concepto: string;
	observaciones?: string;
}

/** Inspección de salida/entrada de una renta */
export interface Inspeccion {
	id: number;
	idRenta: number;
	tipo: string;
	fecha: string;
	kilometraje: string;
	nivelGasolina: string;
	limpieza: string | null;
	tieneRepuesto: boolean;
	tieneGatoCruceta: boolean;
	tieneKitCarretera: boolean;
	tieneDocumentos: boolean;
	danosCarroceria: string | null;
	observaciones: string | null;
}

/** Datos de entrada para registrar una inspección */
export interface InspeccionDatos {
	tipo: string;
	kilometraje: string;
	nivelGasolina: string;
	limpieza?: string;
	tieneRepuesto: boolean;
	tieneGatoCruceta: boolean;
	tieneKitCarretera: boolean;
	tieneDocumentos: boolean;
	danosCarroceria?: string;
	observaciones?: string;
}

/** Renta completa (con pagos e inspecciones) */
export interface Renta {
	id: number;
	/** Número de contrato secuencial e independiente del id (GEN_RENTA_NO_CONTRATO) */
	noContrato: number;
	anioContrato: number;
	placa: string | null;
	idCliente: number | null;
	nombreCliente: string;
	noLicencia: string | null;
	nacionalidad: string | null;
	fechaRecogida: string;
	horaRecogida: string | null;
	ubicacionRecogida: string | null;
	fechaRetorno: string;
	horaRetorno: string | null;
	ubicacionRetorno: string | null;
	diasCalculados: number;
	horasExtras: number;
	valorDia: string;
	valorHoraExtra: string;
	valorDiaExtra: string;
	costoLavado: string;
	costoSilla: string;
	costoRetorno: string;
	costoDomicilio: string;
	costoCables: string;
	costoInversor: string;
	/** Valor de gasolina a cobrar (cliente entrega/recibe sin tanquear) */
	valorGasolina?: string;
	descuento: string;
	subtotal: string;
	impuestos: string;
	cobraIva: boolean;
	/** ¿Tiene comisión esta renta? (checkbox del formulario; false = sin comisión) */
	tieneComision: boolean;
	/** ¿Cobra horas extras al cierre? (checkbox del formulario; false = cortesía) */
	cobrarHorasExtra: boolean;
	/** Valor de la comisión (se resta del total → valor neto) */
	comision: string;
	/** Valor neto = total − comisión (información financiera) */
	valorNeto: string;
	total: string;
	abono: string;
	saldoPendiente: string;
	estado: string;
	observaciones: string | null;
	fechaDevolucionReal: string | null;
	horaDevolucionReal: string | null;
	kmFinal: string | null;
	tanqueFinal: string | null;
	kmSalida: string;
	tanqueSalida: string | null;
	idReserva: number | null;
	createdAt: string | null;
	/** Vehículo (JOIN con autos): marca + modelo */
	vehiculo: string;
	/** Pagos registrados contra la renta */
	pagos: Pago[];
	/** Inspecciones de la renta */
	inspecciones: Inspeccion[];
}

/** Datos de entrada para crear/actualizar una renta (el backend recalcula totales) */
export interface RentaDatos {
	placa?: string | null;
	idCliente?: number | null;
	nombreCliente: string;
	noLicencia?: string;
	nacionalidad?: string;
	fechaRecogida: string;
	horaRecogida?: string;
	ubicacionRecogida?: string;
	fechaRetorno: string;
	horaRetorno?: string;
	ubicacionRetorno?: string;
	diasCalculados: number;
	horasExtras: number;
	valorDia: string;
	valorHoraExtra: string;
	valorDiaExtra: string;
	costoLavado: string;
	costoSilla: string;
	costoRetorno: string;
	costoDomicilio: string;
	costoCables: string;
	costoInversor: string;
	/** Valor de gasolina a cobrar (cliente entrega/recibe sin tanquear) */
	valorGasolina?: string;
	descuento: string;
	subtotal?: string;
	impuestos?: string;
	cobraIva: boolean;
	/** ¿Tiene comisión? (checkbox del formulario) */
	tieneComision: boolean;
	/** ¿Cobra horas extras al cierre? (checkbox del formulario; false = cortesía) */
	cobrarHorasExtra?: boolean;
	/** Valor de la comisión a restar del total */
	comision: string;
	/** Valor neto (calculado por el backend: total − comisión) */
	valorNeto?: string;
	total?: string;
	abono: string;
	saldoPendiente?: string;
	observaciones?: string;
	kmSalida: string;
	tanqueSalida?: string;
	idReserva?: number | null;
}

/** Datos del cierre de una renta (devolución real y ajustes) */
export interface RentaCierreDatos {
	fechaDevolucionReal?: string;
	horaDevolucionReal?: string;
	kmFinal?: string;
	tanqueFinal?: string;
	diasCalculados?: number | null;
	horasExtras?: number | null;
	valorDia?: string;
	valorHoraExtra?: string;
	descuento?: string;
	observaciones?: string;
}

/** Datos para editar una renta cerrada (corrección de errores de digitación) */
export interface RentaCierreEditDatos {
	valorDia?: string;
	valorHoraExtra?: string;
	diasCalculados?: number | null;
	horasExtras?: number | null;
	descuento?: string;
	observaciones?: string;
}

/** Datos para extender una renta activa */
export interface ExtensionDatos {
	/** Tipo de extensión: "horas" o "dias" */
	tipo: string;
	/** Cantidad de horas o días a agregar */
	cantidad: number;
	/** Valor unitario (hora o día extra) */
	valor: string;
	/** Observaciones sobre la extensión */
	observaciones?: string;
}

/** Extensión de una renta (historial) */
export interface ExtensionRenta {
	id: number;
	idRenta: number;
	tipo: string;
	cantidad: number;
	valorUnitario: string;
	valorTotal: string;
	observaciones: string | null;
	usuario: string | null;
	createdAt: string | null;
}

/** Resultado de la cancelación de una renta */
export interface RentaCancelada {
	renta: Renta;
	cancelada: boolean;
}

export const rentaApi = {
	listar: (
		sessionId: string,
		busqueda?: string,
		estado?: string,
		placa?: string,
		fechaDesde?: string,
		fechaHasta?: string
	) =>
		invokeCmd<Renta[]>('listar_rentas', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null,
			placa: placa || null,
			fechaDesde: fechaDesde || null,
			fechaHasta: fechaHasta || null
		}),
	obtener: (sessionId: string, id: number) => invokeCmd<Renta>('obtener_renta', { sessionId, id }),
	crear: (sessionId: string, datos: RentaDatos) =>
		invokeCmd<Renta>('crear_renta', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: RentaDatos) =>
		invokeCmd<Renta>('actualizar_renta', { sessionId, id, datos }),
	cerrar: (sessionId: string, id: number, datos: RentaCierreDatos) =>
		invokeCmd<Renta>('cerrar_renta', { sessionId, id, datos }),
	cambiarAuto: (sessionId: string, id: number, placa: string) =>
		invokeCmd<Renta>('cambiar_auto_renta', { sessionId, id, placa }),
	cancelar: (sessionId: string, id: number) =>
		invokeCmd<RentaCancelada>('cancelar_renta', { sessionId, id }),
	editarCerrada: (sessionId: string, id: number, datos: RentaCierreEditDatos) =>
		invokeCmd<Renta>('editar_renta_cerrada', { sessionId, id, datos }),
	extender: (sessionId: string, id: number, datos: ExtensionDatos) =>
		invokeCmd<Renta>('extender_renta', { sessionId, id, datos }),
	listarExtensiones: (sessionId: string, idRenta: number) =>
		invokeCmd<ExtensionRenta[]>('listar_extensiones', { sessionId, idRenta }),
	eliminar: (sessionId: string, id: number) => invokeCmd<void>('eliminar_renta', { sessionId, id }),
	registrarPago: (sessionId: string, idRenta: number, datos: PagoDatos) =>
		invokeCmd<Pago>('registrar_pago_renta', { sessionId, idRenta, datos }),
	registrarInspeccion: (sessionId: string, idRenta: number, datos: InspeccionDatos) =>
		invokeCmd<Inspeccion>('registrar_inspeccion_renta', { sessionId, idRenta, datos }),
	activas: (sessionId: string) => invokeCmd<Renta[]>('rentas_activas', { sessionId })
};
