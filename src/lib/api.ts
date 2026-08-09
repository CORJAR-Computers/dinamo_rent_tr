// api.ts — Capa de invocación IPC tipada hacia los comandos Tauri (Rust)
import { invoke } from '@tauri-apps/api/core';

/** Formato de error devuelto por el backend (core/error.rs ErrorPayload) */
export interface ApiErrorPayload {
	kind: string;
	message: string;
	detail?: string;
}

/** Error de aplicación con mensaje para usuario final */
export class ApiError extends Error {
	kind: string;
	detail?: string;

	constructor(payload: ApiErrorPayload) {
		super(payload.message);
		this.name = 'ApiError';
		this.kind = payload.kind;
		this.detail = payload.detail;
	}
}

/**
 * Invoca un comando Tauri y normaliza los errores.
 * Si el backend devuelve Err(payload), lanza ApiError con el mensaje de usuario.
 */
export async function invokeCmd<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (err) {
		// Tauri envuelve Err(String) de comandos síncronos
		if (typeof err === 'string') {
			try {
				const parsed = JSON.parse(err) as ApiErrorPayload;
				throw new ApiError(parsed);
			} catch {
				throw new ApiError({ kind: 'generic', message: err });
			}
		}
		// Objeto de error estructurado
		if (err && typeof err === 'object' && 'kind' in err) {
			throw new ApiError(err as ApiErrorPayload);
		}
		throw new ApiError({ kind: 'generic', message: String(err) });
	}
}

// ─── Comandos de autenticación ───────────────────────────────────────────────

export interface LoginResult {
	success: boolean;
	sessionId: string;
	username: string;
	nombre: string | null;
	rol: string | null;
	debeCambiarPassword: boolean;
}

export interface LoginStatus {
	isLocked: boolean;
	lockoutRemainingSeconds: number;
	failedAttempts: number;
	remainingAttempts: number;
}

export interface SessionData {
	userId: number;
	username: string;
	role: string;
	nombre: string;
	debeCambiarPassword: boolean;
}

export const authApi = {
	login: (username: string, password: string) =>
		invokeCmd<LoginResult>('login', { username, password }),
	logout: (sessionId: string) => invokeCmd<void>('logout', { sessionId }),
	changePassword: (username: string, currentPassword: string, newPassword: string) =>
		invokeCmd<void>('change_password', { username, currentPassword, newPassword }),
	getLoginStatus: (username: string) =>
		invokeCmd<LoginStatus>('get_login_status', { username }),
	getSession: (sessionId: string) => invokeCmd<SessionData>('get_session', { sessionId }),
	unlockAccount: (sessionId: string, username: string) =>
		invokeCmd<boolean>('unlock_account', { sessionId, username }),
	/** Preferencia de tema del usuario ('light' | 'dark' | 'auto' | null) */
	obtenerTema: (sessionId: string) => invokeCmd<string | null>('obtener_tema', { sessionId }),
	guardarTema: (sessionId: string, tema: string) =>
		invokeCmd<void>('guardar_tema', { sessionId, tema })
};

// ─── Comandos de negocio (requieren sessionId) ───────────────────────────────

/** Vehículo (repositories/auto.rs) */
export interface Auto {
	placa: string;
	marca: string;
	modelo: string;
	version: string | null;
	color: string | null;
	tipo: string;
	cilindraje: string | null;
	transmision: string | null;
	combustible: string | null;
	noMotor: string | null;
	noChasis: string | null;
	propietario: string | null;
	estado: string;
	costoFijoMensual: string;
	kilometraje: number;
	ubicacion: string | null;
	tipoAdquisicion: string | null;
	proximoAceite: number | null;
	proximoFrenos: number | null;
	vencimientoSoat: string | null;
	vencimientoTecnico: string | null;
	vencimientoExtintor: string | null;
	vencimientoBateria: string | null;
	observaciones: string | null;
	fechaIngreso: string;
	createdAt: string | null;
}

/** Datos de entrada para crear/actualizar un vehículo */
export interface AutoDatos {
	placa: string;
	marca: string;
	modelo: string;
	version?: string;
	color?: string;
	tipo: string;
	cilindraje?: string;
	transmision?: string;
	combustible?: string;
	noMotor?: string;
	noChasis?: string;
	propietario?: string;
	estado: string;
	costoFijoMensual: string;
	kilometraje: number;
	ubicacion?: string;
	tipoAdquisicion?: string;
	proximoAceite?: number | null;
	proximoFrenos?: number | null;
	vencimientoSoat?: string;
	vencimientoTecnico?: string;
	vencimientoExtintor?: string;
	vencimientoBateria?: string;
	observaciones?: string;
	fechaIngreso: string;
}

/** Alerta de vencimiento de un vehículo (services/auto.rs) */
export interface AlertaVencimiento {
	placa: string;
	marca: string;
	modelo: string;
	tipo: string;
	fecha: string | null;
	diasRestantes: number | null;
	detalle: string;
	critica: boolean;
}

export const autoApi = {
	listar: (sessionId: string, busqueda?: string, estado?: string) =>
		invokeCmd<Auto[]>('listar_autos', { sessionId, busqueda: busqueda || null, estado: estado || null }),
	obtener: (sessionId: string, placa: string) =>
		invokeCmd<Auto>('obtener_auto', { sessionId, placa }),
	crear: (sessionId: string, datos: AutoDatos) =>
		invokeCmd<Auto>('crear_auto', { sessionId, datos }),
	actualizar: (sessionId: string, placa: string, datos: AutoDatos) =>
		invokeCmd<Auto>('actualizar_auto', { sessionId, placa, datos }),
	eliminar: (sessionId: string, placa: string) =>
		invokeCmd<void>('eliminar_auto', { sessionId, placa }),
	alertas: (sessionId: string) => invokeCmd<AlertaVencimiento[]>('alertas_autos', { sessionId })
};

/** Cliente (repositories/cliente.rs) — PII ya descifrada por el backend */
export interface Cliente {
	id: number;
	tipoDoc: string | null;
	noDoc: string | null;
	nombres: string;
	apellidos: string | null;
	nombreCompleto: string;
	celular: string | null;
	celular2: string | null;
	email: string | null;
	ciudad: string | null;
	estadoRegion: string | null;
	pais: string | null;
	nacionalidad: string | null;
	dirResidencia: string | null;
	dirTemporal: string | null;
	hotel: string | null;
	habitacion: string | null;
	noLicencia: string | null;
	tipoLicencia: string | null;
	vencimientoLicencia: string | null;
	estado: string;
	createdAt: string | null;
}

/** Cliente con metadatos de descifrado PII */
export interface ClienteConPii {
	cliente: Cliente;
	piiOculto: boolean;
}

/** Datos de entrada para crear/actualizar un cliente (PII en claro) */
export interface ClienteDatos {
	tipoDoc?: string;
	noDoc?: string;
	nombres: string;
	apellidos?: string;
	celular?: string;
	celular2?: string;
	email?: string;
	ciudad?: string;
	estadoRegion?: string;
	pais?: string;
	nacionalidad?: string;
	dirResidencia?: string;
	dirTemporal?: string;
	hotel?: string;
	habitacion?: string;
	noLicencia?: string;
	tipoLicencia?: string;
	vencimientoLicencia?: string;
	estado: string;
}

export const clienteApi = {
	listar: (sessionId: string, busqueda?: string, estado?: string) =>
		invokeCmd<ClienteConPii[]>('listar_clientes', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null
		}),
	obtener: (sessionId: string, id: number) => invokeCmd<ClienteConPii>('obtener_cliente', { sessionId, id }),
	crear: (sessionId: string, datos: ClienteDatos) =>
		invokeCmd<ClienteConPii>('crear_cliente', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: ClienteDatos) =>
		invokeCmd<ClienteConPii>('actualizar_cliente', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) => invokeCmd<void>('eliminar_cliente', { sessionId, id })
};

/** Listas de negocio configurables (commands/business.rs) */
export interface BusinessLists {
	tiposAuto: string[];
	tiposTransmision: string[];
	tiposCombustible: string[];
	estadosAuto: string[];
	tiposAdquisicion: string[];
	tiposDoc: string[];
	estadosCliente: string[];
	estadosReserva: string[];
	tiposGasto: string[];
	nivelTanque: string[];
	tiposMantenimiento: string[];
	rolesConInformes: string[];
	rolesConUsuarios: string[];
	rolesDisponibles: string[];
}

export const businessApi = {
	listas: (sessionId: string) => invokeCmd<BusinessLists>('get_business_lists', { sessionId })
};

// ─── Clave PII (db_encryption_key) ────────────────────────────────────────────

/** Análisis de descifrado de los datos legacy (services/pii.rs) */
export interface PiiAnalisis {
	claveConfigurada: boolean;
	totalClientes: number;
	clientesLegacy: number;
	clientesDescifrados: number;
	clientesOcultos: number;
	muestra: { cliente: string; campo: string; valor: string } | null;
}

/** Resultado de guardar/eliminar la clave */
export interface ClaveGuardada {
	claveConfigurada: boolean;
	analisis: PiiAnalisis;
}

export const piiApi = {
	/** Estado actual: ¿hay clave? ¿cuántos clientes legacy se descifran? */
	status: (sessionId: string) => invokeCmd<PiiAnalisis>('get_pii_status', { sessionId }),
	/** Prueba una clave candidata sin guardarla */
	probar: (sessionId: string, clave: string) =>
		invokeCmd<PiiAnalisis>('probar_clave_pii', { sessionId, clave }),
	/** Guarda la clave en config.ini y la aplica en caliente */
	guardar: (sessionId: string, clave: string) =>
		invokeCmd<ClaveGuardada>('guardar_clave_pii', { sessionId, clave }),
	/** Elimina la clave configurada */
	eliminar: (sessionId: string) => invokeCmd<ClaveGuardada>('eliminar_clave_pii', { sessionId })
};

/** Datos del Dashboard (services/dashboard.rs) */
export interface DashboardData {
	totalAutos: number;
	autosPorEstado: { estado: string; total: number }[];
	totalClientes: number;
	clientesRecientes: Cliente[];
	alertas: AlertaVencimiento[];
	rentasActivas: number;
	piiKeyConfigurada: boolean;
}

export const dashboardApi = {
	getData: (sessionId: string) => invokeCmd<DashboardData>('get_dashboard_data', { sessionId })
};

// ─── Comandos de Reservas ────────────────────────────────────────────────────

/** Reserva (repositories/reserva.rs) */
export interface Reserva {
	id: number;
	idCliente: number | null;
	nombreCliente: string;
	nacionalidad: string | null;
	categoriaVehiculo: string | null;
	placaAsignada: string | null;
	fechaRecogida: string;
	horaRecogida: string | null;
	ubicacionRecogida: string | null;
	fechaRetorno: string;
	horaRetorno: string | null;
	ubicacionRetorno: string | null;
	diasCalculados: number;
	horasExtras: number;
	valorDia: string;
	valorHoraAdic: string;
	abono: string;
	total: string;
	observaciones: string | null;
	estado: string;
	createdAt: string | null;
	updatedAt: string | null;
}

/** Datos de entrada para crear/actualizar una reserva */
export interface ReservaDatos {
	idCliente?: number | null;
	nombreCliente: string;
	nacionalidad?: string;
	categoriaVehiculo?: string;
	placaAsignada?: string;
	fechaRecogida: string;
	horaRecogida?: string;
	ubicacionRecogida?: string;
	fechaRetorno: string;
	horaRetorno?: string;
	ubicacionRetorno?: string;
	diasCalculados: number;
	horasExtras: number;
	valorDia: string;
	valorHoraAdic: string;
	abono: string;
	total: string;
	observaciones?: string;
	estado: string;
}

/** Resultado de la cancelación de una reserva */
export interface ReservaCancelada {
	reserva: Reserva;
	cancelada: boolean;
}

// ─── Comandos de Gastos (caja menor) ────────────────────────────────────────

/** Gasto (repositories/gasto.rs) */
export interface Gasto {
	id: number;
	placa: string | null;
	fecha: string;
	categoria: string;
	descripcion: string;
	monto: string;
	comprobante: string | null;
	usuario: string | null;
	createdAt: string | null;
	updatedAt: string | null;
}

/** Datos de entrada para crear/actualizar un gasto */
export interface GastoDatos {
	placa?: string;
	fecha: string;
	categoria: string;
	descripcion: string;
	monto: string;
	comprobante?: string;
}

/** Total por placa o categoría */
export interface TotalGasto {
	clave: string;
	total: string;
}

/** Resumen de totales de gastos (services/gasto.rs) */
export interface TotalesGastos {
	totalGeneral: string;
	totalMes: string;
	porPlaca: TotalGasto[];
	porCategoria: TotalGasto[];
}

export const gastoApi = {
	listar: (sessionId: string, busqueda?: string, placa?: string, categoria?: string) =>
		invokeCmd<Gasto[]>('listar_gastos', {
			sessionId,
			busqueda: busqueda || null,
			placa: placa || null,
			categoria: categoria || null
		}),
	recientes: (sessionId: string, limit?: number) =>
		invokeCmd<Gasto[]>('gastos_recientes', { sessionId, limit: limit ?? null }),
	obtener: (sessionId: string, id: number) => invokeCmd<Gasto>('obtener_gasto', { sessionId, id }),
	crear: (sessionId: string, datos: GastoDatos) =>
		invokeCmd<Gasto>('crear_gasto', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: GastoDatos) =>
		invokeCmd<Gasto>('actualizar_gasto', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_gasto', { sessionId, id }),
	totales: (sessionId: string) => invokeCmd<TotalesGastos>('totales_gastos', { sessionId })
};

// ─── Comandos de Mantenimiento de vehículos ──────────────────────────────────

/** Registro de mantenimiento (repositories/mantenimiento.rs) */
export interface Mantenimiento {
	id: number;
	placa: string;
	vehiculo: string;
	tipo: string;
	fecha: string;
	descripcion: string | null;
	observaciones: string | null;
	costo: string;
	kmProximoCambioAceite: number | null;
	total: string;
	createdAt: string | null;
	updatedAt: string | null;
}

/** Datos de entrada para crear/actualizar un mantenimiento */
export interface MantenimientoDatos {
	placa: string;
	tipo: string;
	fecha: string;
	descripcion?: string;
	observaciones?: string;
	costo: string;
	kmProximoCambioAceite?: number | null;
}

/** Total por placa o tipo */
export interface TotalMantenimiento {
	clave: string;
	total: string;
}

/** Resumen de totales de mantenimiento (services/mantenimiento.rs) */
export interface TotalesMantenimiento {
	totalGeneral: string;
	porPlaca: TotalMantenimiento[];
	porTipo: TotalMantenimiento[];
}

/** Alerta por kilometraje (cambio de aceite o frenos próximo/vencido) */
export interface AlertaKm {
	placa: string;
	marca: string;
	modelo: string;
	tipo: string;
	kmActual: number;
	kmProximo: number;
	kmRestante: number;
	critica: boolean;
}

export const mantenimientoApi = {
	listar: (sessionId: string, busqueda?: string, placa?: string, tipo?: string) =>
		invokeCmd<Mantenimiento[]>('listar_mantenimientos', {
			sessionId,
			busqueda: busqueda || null,
			placa: placa || null,
			tipo: tipo || null
		}),
	recientes: (sessionId: string, limit?: number) =>
		invokeCmd<Mantenimiento[]>('mantenimientos_recientes', { sessionId, limit: limit ?? null }),
	obtener: (sessionId: string, id: number) =>
		invokeCmd<Mantenimiento>('obtener_mantenimiento', { sessionId, id }),
	crear: (sessionId: string, datos: MantenimientoDatos) =>
		invokeCmd<Mantenimiento>('crear_mantenimiento', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: MantenimientoDatos) =>
		invokeCmd<Mantenimiento>('actualizar_mantenimiento', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_mantenimiento', { sessionId, id }),
	totales: (sessionId: string) =>
		invokeCmd<TotalesMantenimiento>('totales_mantenimiento', { sessionId }),
	alertasKm: (sessionId: string) =>
		invokeCmd<AlertaKm[]>('alertas_km_mantenimiento', { sessionId })
};

// ─── Vista de Auditoría (solo administradores) ───────────────────────────────

/** Evento de auditoría (repositories/auditoria.rs) */
export interface AuditoriaEvento {
	id: number;
	usuario: string;
	accion: string;
	mensaje: string | null;
	ip: string;
	fecha: string;
}

/** Resultado paginado de la consulta de auditoría */
export interface AuditoriaResultado {
	eventos: AuditoriaEvento[];
	total: number;
	pagina: number;
	porPagina: number;
}

export const auditoriaApi = {
	/** Lista eventos con filtros opcionales y paginación (solo admin) */
	listar: (
		sessionId: string,
		filtros: {
			usuario?: string;
			accion?: string;
			fechaDesde?: string;
			fechaHasta?: string;
			busqueda?: string;
		},
		pagina?: number,
		porPagina?: number
	) =>
		invokeCmd<AuditoriaResultado>('listar_auditoria', {
			sessionId,
			usuario: filtros.usuario || null,
			accion: filtros.accion || null,
			fechaDesde: filtros.fechaDesde || null,
			fechaHasta: filtros.fechaHasta || null,
			busqueda: filtros.busqueda || null,
			pagina: pagina ?? 1,
			porPagina: porPagina ?? 50
		}),
	/** Acciones distintas disponibles para el filtro (solo admin) */
	acciones: (sessionId: string) =>
		invokeCmd<string[]>('acciones_auditoria', { sessionId }),
	/** Usuarios distintos disponibles para el filtro (solo admin) */
	usuarios: (sessionId: string) =>
		invokeCmd<string[]>('usuarios_auditoria', { sessionId })
};

export const reservaApi = {
	listar: (sessionId: string, busqueda?: string, estado?: string) =>
		invokeCmd<Reserva[]>('listar_reservas', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null
		}),
	proximas: (sessionId: string, limit?: number) =>
		invokeCmd<Reserva[]>('proximas_reservas', { sessionId, limit: limit ?? null }),
	obtener: (sessionId: string, id: number) => invokeCmd<Reserva>('obtener_reserva', { sessionId, id }),
	crear: (sessionId: string, datos: ReservaDatos) =>
		invokeCmd<Reserva>('crear_reserva', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: ReservaDatos) =>
		invokeCmd<Reserva>('actualizar_reserva', { sessionId, id, datos }),
	cancelar: (sessionId: string, id: number) =>
		invokeCmd<ReservaCancelada>('cancelar_reserva', { sessionId, id }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_reserva', { sessionId, id })
};

// ─── Comandos de Rentas ─────────────────────────────────────────────────────

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
	descuento: string;
	subtotal: string;
	impuestos: string;
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
	descuento: string;
	subtotal?: string;
	impuestos?: string;
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

/** Resultado de la cancelación de una renta */
export interface RentaCancelada {
	renta: Renta;
	cancelada: boolean;
}

export const rentaApi = {
	listar: (sessionId: string, busqueda?: string, estado?: string, placa?: string) =>
		invokeCmd<Renta[]>('listar_rentas', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null,
			placa: placa || null
		}),
	obtener: (sessionId: string, id: number) => invokeCmd<Renta>('obtener_renta', { sessionId, id }),
	crear: (sessionId: string, datos: RentaDatos) =>
		invokeCmd<Renta>('crear_renta', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: RentaDatos) =>
		invokeCmd<Renta>('actualizar_renta', { sessionId, id, datos }),
	cerrar: (sessionId: string, id: number, datos: RentaCierreDatos) =>
		invokeCmd<Renta>('cerrar_renta', { sessionId, id, datos }),
	cancelar: (sessionId: string, id: number) =>
		invokeCmd<RentaCancelada>('cancelar_renta', { sessionId, id }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_renta', { sessionId, id }),
	registrarPago: (sessionId: string, idRenta: number, datos: PagoDatos) =>
		invokeCmd<Pago>('registrar_pago_renta', { sessionId, idRenta, datos }),
	registrarInspeccion: (sessionId: string, idRenta: number, datos: InspeccionDatos) =>
		invokeCmd<Inspeccion>('registrar_inspeccion_renta', { sessionId, idRenta, datos }),
	activas: (sessionId: string) => invokeCmd<Renta[]>('rentas_activas', { sessionId })
};

// ─── Comandos de Comparendos ─────────────────────────────────────────────────

/** Comparendo (repositories/comparendo.rs) */
export interface Comparendo {
	id: number;
	placa: string;
	vehiculo: string;
	fechaInfraccion: string;
	horaInfraccion: string;
	monto: string;
	idRenta: number | null;
	idCliente: number | null;
	estado: string;
	observaciones: string | null;
	createdAt: string | null;
	updatedAt: string | null;
}

/** Datos de entrada para crear/actualizar un comparendo */
export interface ComparendoDatos {
	placa: string;
	fechaInfraccion: string;
	horaInfraccion: string;
	monto: string;
	idRenta?: number | null;
	idCliente?: number | null;
	estado: string;
	observaciones?: string;
}

/** Total por placa o estado */
export interface TotalComparendo {
	clave: string;
	total: string;
}

/** Resumen de totales de comparendos (services/comparendo.rs) */
export interface TotalesComparendos {
	totalGeneral: string;
	totalPendiente: string;
	porPlaca: TotalComparendo[];
	porEstado: TotalComparendo[];
}

export const comparendoApi = {
	listar: (sessionId: string, busqueda?: string, placa?: string, estado?: string) =>
		invokeCmd<Comparendo[]>('listar_comparendos', {
			sessionId,
			busqueda: busqueda || null,
			placa: placa || null,
			estado: estado || null
		}),
	obtener: (sessionId: string, id: number) =>
		invokeCmd<Comparendo>('obtener_comparendo', { sessionId, id }),
	crear: (sessionId: string, datos: ComparendoDatos) =>
		invokeCmd<Comparendo>('crear_comparendo', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: ComparendoDatos) =>
		invokeCmd<Comparendo>('actualizar_comparendo', { sessionId, id, datos }),
	marcarPagado: (sessionId: string, id: number) =>
		invokeCmd<Comparendo>('marcar_pagado_comparendo', { sessionId, id }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_comparendo', { sessionId, id }),
	totales: (sessionId: string) =>
		invokeCmd<TotalesComparendos>('totales_comparendos', { sessionId })
};

// ─── Comandos de Informes ───────────────────────────────────────────────────

/** Detalle de una renta del mes (services/informe.rs) */
export interface RentaInforme {
	id: number;
	placa: string;
	nombreCliente: string;
	total: string;
	estado: string;
	fechaRecogida: string;
}

/** Utilidad de un vehículo en el mes (services/informe.rs) */
export interface UtilidadVehiculo {
	placa: string;
	vehiculo: string;
	ingresos: string;
	costos: string;
	utilidad: string;
}

/** Balance mensual (services/informe.rs) */
export interface InformeMensual {
	fechaInicio: string;
	fechaFin: string;
	ingresosPagos: string;
	ingresosReservas: string;
	totalIngresos: string;
	egresosGastos: string;
	egresosMantenimiento: string;
	egresosComparendos: string;
	totalEgresos: string;
	balance: string;
	gastosPorCategoria: [string, string][];
	rentas: RentaInforme[];
	utilidadPorVehiculo: UtilidadVehiculo[];
}

export const informeApi = {
	mensual: (sessionId: string, fechaInicio: string, fechaFin: string) =>
		invokeCmd<InformeMensual>('informe_mensual', { sessionId, fechaInicio, fechaFin })
};

// ─── Comandos de Usuarios (solo roles de administración) ────────────────────

/** Usuario (repositories/usuario.rs) — sin contraseña */
export interface Usuario {
	id: number;
	username: string;
	nombre: string | null;
	rol: string | null;
	email: string | null;
	activo: boolean;
	debeCambiarPassword: boolean;
	intentosFallidos: number;
	ultimoAcceso: string | null;
	createdAt: string | null;
}

/** Datos para crear un usuario (contraseña inicial) */
export interface UsuarioDatos {
	username: string;
	password: string;
	nombre: string;
	rol: string;
	email?: string;
	activo: boolean;
	debeCambiarPassword: boolean;
}

/** Datos para actualizar un usuario (sin contraseña) */
export interface UsuarioDatosActualizar {
	nombre: string;
	rol: string;
	email?: string;
	activo: boolean;
}

/** Resultado del cambio forzado de contraseña */
export interface UsuarioConCambio {
	usuario: Usuario;
	cambioForzado: boolean;
}

export const usuarioApi = {
	listar: (sessionId: string, busqueda?: string) =>
		invokeCmd<Usuario[]>('listar_usuarios', { sessionId, busqueda: busqueda || null }),
	crear: (sessionId: string, datos: UsuarioDatos) =>
		invokeCmd<Usuario>('crear_usuario', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: UsuarioDatosActualizar) =>
		invokeCmd<Usuario>('actualizar_usuario', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_usuario', { sessionId, id }),
	forzarCambioPassword: (sessionId: string, id: number, nuevaPassword: string) =>
		invokeCmd<UsuarioConCambio>('forzar_cambio_password_usuario', { sessionId, id, nuevaPassword }),
	desbloquear: (sessionId: string, username: string) =>
		invokeCmd<boolean>('desbloquear_usuario', { sessionId, username })
};
