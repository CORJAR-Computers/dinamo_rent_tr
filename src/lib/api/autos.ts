// autos.ts — Comandos y tipos para vehículos
import { invokeCmd } from './base';

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
		invokeCmd<Auto[]>('listar_autos', {
			sessionId,
			busqueda: busqueda || null,
			estado: estado || null
		}),
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
