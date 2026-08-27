// usuarios.ts — Comandos y tipos para gestión de usuarios (solo roles de administración)
import { invokeCmd } from './base';

/** Usuario (repositories/usuario.rs) — sin contraseña */
export interface Usuario {
	id: number;
	username: string;
	nombre: string | null;
	rol: string | null;
	email: string | null;
	activo: boolean;
	debeCambiarPassword: boolean;
	intentosFallidos: number;
	ultimoAcceso: string | null;
	createdAt: string | null;
}

/** Datos para crear un usuario (contraseña inicial) */
export interface UsuarioDatos {
	username: string;
	password: string;
	nombre: string;
	rol: string;
	email?: string;
	activo: boolean;
	debeCambiarPassword: boolean;
}

/** Datos para actualizar un usuario (sin contraseña) */
export interface UsuarioDatosActualizar {
	nombre: string;
	rol: string;
	email?: string;
	activo: boolean;
}

/** Resultado del cambio forzado de contraseña */
export interface UsuarioConCambio {
	usuario: Usuario;
	cambioForzado: boolean;
}

export const usuarioApi = {
	listar: (sessionId: string, busqueda?: string) =>
		invokeCmd<Usuario[]>('listar_usuarios', { sessionId, busqueda: busqueda || null }),
	crear: (sessionId: string, datos: UsuarioDatos) =>
		invokeCmd<Usuario>('crear_usuario', { sessionId, datos }),
	actualizar: (sessionId: string, id: number, datos: UsuarioDatosActualizar) =>
		invokeCmd<Usuario>('actualizar_usuario', { sessionId, id, datos }),
	eliminar: (sessionId: string, id: number) =>
		invokeCmd<void>('eliminar_usuario', { sessionId, id }),
	forzarCambioPassword: (sessionId: string, id: number, nuevaPassword: string) =>
		invokeCmd<UsuarioConCambio>('forzar_cambio_password_usuario', { sessionId, id, nuevaPassword }),
	desbloquear: (sessionId: string, username: string) =>
		invokeCmd<boolean>('desbloquear_usuario', { sessionId, username })
};
