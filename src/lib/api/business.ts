// business.ts — Listas de negocio configurables
import { invokeCmd } from './base';

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
	rolesConEliminar: string[];
	rolesDisponibles: string[];
	impuestoPorcentaje: number;
}

export const businessApi = {
	listas: (sessionId: string) => invokeCmd<BusinessLists>('get_business_lists', { sessionId })
};
