// mantenimiento.ts — Comandos y tipos para mantenimiento de vehículos
import { invokeCmd } from './base';

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
	alertasKm: (sessionId: string) => invokeCmd<AlertaKm[]>('alertas_km_mantenimiento', { sessionId })
};
