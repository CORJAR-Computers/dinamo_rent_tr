// empresa.ts — Comandos y tipos para configuración de la empresa (datos y logo)
import { invokeCmd } from './base';

/** Configuración de la empresa (repositories/empresa.rs) */
export interface EmpresaConfig {
	nombre: string | null;
	nit: string | null;
	direccion: string | null;
	telefono: string | null;
	email: string | null;
	web: string | null;
	ciudad: string | null;
	/** Data URL del logo (data:image/...;base64,...) o null */
	logo: string | null;
}

/** Datos para guardar la configuración (logo como data URL o null para quitar) */
export interface EmpresaConfigDatos {
	nombre?: string | null;
	nit?: string | null;
	direccion?: string | null;
	telefono?: string | null;
	email?: string | null;
	web?: string | null;
	ciudad?: string | null;
	logo?: string | null;
}

export const empresaApi = {
	/** Vista pública (login / menú lateral): solo nombre + logo, sin sesión */
	publica: () => invokeCmd<EmpresaConfig>('empresa_publica'),
	/** Configuración completa (requiere sesión activa) */
	obtener: (sessionId: string) => invokeCmd<EmpresaConfig>('obtener_empresa', { sessionId }),
	/** Guarda los datos + logo (rol de administración de usuarios) */
	guardar: (sessionId: string, datos: EmpresaConfigDatos) =>
		invokeCmd<EmpresaConfig>('guardar_empresa', { sessionId, datos })
};
