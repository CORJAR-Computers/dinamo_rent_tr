// gastos.ts — Comandos y tipos para gastos (caja menor)
import { invokeCmd } from './base';

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
	eliminar: (sessionId: string, id: number) => invokeCmd<void>('eliminar_gasto', { sessionId, id }),
	totales: (sessionId: string) => invokeCmd<TotalesGastos>('totales_gastos', { sessionId })
};
