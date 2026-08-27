// auditoria.ts — Comandos y tipos para auditoría (solo administradores)
import { invokeCmd } from './base';

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
	acciones: (sessionId: string) => invokeCmd<string[]>('acciones_auditoria', { sessionId }),
	/** Usuarios distintos disponibles para el filtro (solo admin) */
	usuarios: (sessionId: string) => invokeCmd<string[]>('usuarios_auditoria', { sessionId })
};
