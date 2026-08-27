// reservas.ts — Comandos y tipos para reservas
import { invokeCmd } from './base';

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
	costoLavado: string;
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
	costoLavado: string;
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

export const reservaApi = {
	listar: (
		sessionId: string,
		busqueda?: string,
		estado?: string,
		fechaDesde?: string,
		fechaHasta?: string
	) =>
		invokeCmd<Reserva[]>('listar_reservas', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null,
			fechaDesde: fechaDesde || null,
			fechaHasta: fechaHasta || null
		}),
	proximas: (sessionId: string, limit?: number) =>
		invokeCmd<Reserva[]>('proximas_reservas', { sessionId, limit: limit ?? null }),
	obtener: (sessionId: string, id: number) =>
		invokeCmd<Reserva>('obtener_reserva', { sessionId, id }),
	crear: (sessionId: string, datos: ReservaDatos) =>
		invokeCmd<Reserva>('crear_reserva', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: ReservaDatos) =>
		invokeCmd<Reserva>('actualizar_reserva', { sessionId, id, datos }),
	cancelar: (sessionId: string, id: number) =>
		invokeCmd<ReservaCancelada>('cancelar_reserva', { sessionId, id }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_reserva', { sessionId, id })
};
