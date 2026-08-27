// informes.ts — Comandos y tipos para reportes contables e informes de rentabilidad
import { invokeCmd } from './base';

/** Detalle de una renta del mes (services/informe.rs) */
export interface RentaInforme {
	id: number;
	placa: string;
	nombreCliente: string;
	total: string;
	comision: string;
	valorNeto: string;
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
	totalComisiones: string;
	ingresosNetos: string;
	balanceNeto: string;
	gastosPorCategoria: [string, string][];
	rentas: RentaInforme[];
	utilidadPorVehiculo: UtilidadVehiculo[];
}

export const informeApi = {
	mensual: (sessionId: string, fechaInicio: string, fechaFin: string) =>
		invokeCmd<InformeMensual>('informe_mensual', { sessionId, fechaInicio, fechaFin })
};
