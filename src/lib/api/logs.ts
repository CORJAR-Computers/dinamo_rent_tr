// logs.ts — Comandos para logs del sistema y frontend
import { invokeCmd } from './base';

export const logApi = {
	/** Lee las últimas N líneas del log principal (admin only) */
	leer: (sessionId: string, lineas?: number) =>
		invokeCmd<string>('leer_logs', { sessionId, lineas: lineas ?? 500 }),
	/** Lee los errores del frontend (admin only) */
	erroresFrontend: (sessionId: string, lineas?: number) =>
		invokeCmd<string>('leer_errores_frontend', { sessionId, lineas: lineas ?? 200 }),
	/** Registra un error del frontend */
	registrarError: (
		sessionId: string,
		mensaje: string,
		stack?: string,
		url?: string,
		linea?: number,
		columna?: number
	) =>
		invokeCmd<void>('registrar_error_frontend', {
			sessionId,
			mensaje,
			stack: stack || null,
			url: url || null,
			linea: linea ?? null,
			columna: columna ?? null
		}),
	/** Exporta todos los logs como texto */
	exportar: (sessionId: string) => invokeCmd<string>('exportar_logs', { sessionId }),
	/** Trunca los archivos de log (admin only) */
	limpiar: (sessionId: string) => invokeCmd<number>('limpiar_logs', { sessionId })
};
