// auth.ts — Comandos de autenticación y sesión
import { invokeCmd } from './base';

export interface LoginResult {
	success: boolean;
	sessionId: string;
	username: string;
	nombre: string | null;
	rol: string | null;
	debeCambiarPassword: boolean;
}

export interface LoginStatus {
	isLocked: boolean;
	lockoutRemainingSeconds: number;
	failedAttempts: number;
	remainingAttempts: number;
}

export interface SessionData {
	userId: number;
	username: string;
	role: string;
	nombre: string;
	debeCambiarPassword: boolean;
}

export const authApi = {
	login: (username: string, password: string) =>
		invokeCmd<LoginResult>('login', { username, password }),
	logout: (sessionId: string) => invokeCmd<void>('logout', { sessionId }),
	changePassword: (username: string, currentPassword: string, newPassword: string) =>
		invokeCmd<void>('change_password', { username, currentPassword, newPassword }),
	getLoginStatus: (username: string) => invokeCmd<LoginStatus>('get_login_status', { username }),
	getSession: (sessionId: string) => invokeCmd<SessionData>('get_session', { sessionId }),
	/** Preferencia de tema del usuario ('light' | 'dark' | 'auto' | null) */
	obtenerTema: (sessionId: string) => invokeCmd<string | null>('obtener_tema', { sessionId }),
	guardarTema: (sessionId: string, tema: string) =>
		invokeCmd<void>('guardar_tema', { sessionId, tema })
};
