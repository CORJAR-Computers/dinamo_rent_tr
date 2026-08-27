// clientes.ts — Comandos y tipos para clientes y clave PII
import { invokeCmd } from './base';

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
	obtener: (sessionId: string, id: number) =>
		invokeCmd<ClienteConPii>('obtener_cliente', { sessionId, id }),
	crear: (sessionId: string, datos: ClienteDatos) =>
		invokeCmd<ClienteConPii>('crear_cliente', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: ClienteDatos) =>
		invokeCmd<ClienteConPii>('actualizar_cliente', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_cliente', { sessionId, id })
};

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
