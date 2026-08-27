// comparendos.ts — Comandos y tipos para multas/comparendos y agente SIMIT
import { invokeCmd } from './base';

/** Responsable del vehículo el día de la infracción (cruce con rentas, backend) */
export interface ResponsableComparendo {
	idRenta: number;
	nombreCliente: string;
	noContrato: number;
	anioContrato: number;
	fechaRecogida: string;
	fechaRetorno: string;
	estadoRenta: string;
}

/** Comparendo (repositories/comparendo.rs) */
export interface Comparendo {
	id: number;
	placa: string;
	vehiculo: string;
	fechaInfraccion: string;
	horaInfraccion: string;
	monto: string;
	numeroComparendo: string | null;
	idRenta: number | null;
	idCliente: number | null;
	estado: string;
	observaciones: string | null;
	createdAt: string | null;
	updatedAt: string | null;
	/** Procedencia: 'SIMIT' (Agente automático) o 'Manual' */
	origen: string;
	/** Última vez que el Agente SIMIT confirmó que existe en el portal */
	ultimoVistoSimit: string | null;
	responsable: ResponsableComparendo | null;
}

/** Datos de entrada para crear/actualizar un comparendo */
export interface ComparendoDatos {
	placa: string;
	fechaInfraccion: string;
	horaInfraccion: string;
	monto: string;
	numeroComparendo?: string | null;
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
	listar: (
		sessionId: string,
		busqueda?: string,
		placa?: string,
		estado?: string,
		noConfirmados?: boolean
	) =>
		invokeCmd<Comparendo[]>('listar_comparendos', {
			sessionId,
			busqueda: busqueda || null,
			placa: placa || null,
			estado: estado || null,
			noConfirmados: noConfirmados || null
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

/** Registro de comparendo/multa tal como lo devuelve el SIMIT (services/simit.rs) */
export interface RegistroSimit {
	numero: string | null;
	placa: string;
	fechaInfraccion: string;
	horaInfraccion: string;
	monto: string;
	estado: string;
	organismo: string;
	codigoInfraccion: string;
	descripcion: string;
	esComparendo: boolean;
	/** true = se insertó en esta sincronización; false = ya estaba en la BD */
	nuevo: boolean;
	/** id en la tabla comparendos (para marcar en la lista cuál es nuevo) */
	id: number | null;
}

/** Error de una placa durante la sincronización */
export interface ErrorPlacaSimit {
	placa: string;
	error: string;
}

/** Métricas de rendimiento de la sincronización */
export interface MetricasSimit {
	tiempoTotalMs: number;
	tiempoPromedioPlacaMs: number;
	tiempoCaptchaMs: number;
	tiempoConsultaMs: number;
	totalReintentos: number;
	circuitBreakerState: string;
	placasExitosas: number;
	placasTimeout: number;
	placasErrorRed: number;
}

/** Resultado de una sincronización con el SIMIT */
export interface ResultadoSincronizacion {
	sincronizadoEn: string;
	placasConsultadas: number;
	placasConError: number;
	encontrados: number;
	insertados: number;
	duplicados: number;
	totalPendiente: string;
	registros: RegistroSimit[];
	errores: ErrorPlacaSimit[];
	reporteHtml: string | null;
	metricas: MetricasSimit;
}

/** Evento de progreso durante la sincronización */
export interface EventoProgresoSimit {
	tipo: string;
	placaActual: string | null;
	progreso: number;
	mensaje: string;
	timestamp: string;
	indicePlaca: number;
	totalPlacas: number;
}

/** Nivel de severidad del log SIMIT */
export type LogLevelSimit = 'info' | 'success' | 'warn' | 'error';

/** Evento de log en tiempo real durante la sincronización */
export interface EventoLogSimit {
	timestamp: string;
	level: LogLevelSimit;
	message: string;
	placa: string | null;
	detail: string | null;
}

/** Estado en memoria del agente (habilitado, intervalos, última corrida) */
export interface InfoAgenteSimit {
	habilitado: boolean;
	intervalHours: number;
	startDelayMinutes: number;
	ejecutando: boolean;
	ultimaSincronizacion: string | null;
	proximaSincronizacion: string | null;
	ultimoResultado: ResultadoSincronizacion | null;
	ultimoError: string | null;
}

export const simitApi = {
	estado: (sessionId: string) => invokeCmd<InfoAgenteSimit>('simit_sync_status', { sessionId }),
	sincronizarAhora: (sessionId: string) =>
		invokeCmd<ResultadoSincronizacion>('simit_sync_now', { sessionId })
};
