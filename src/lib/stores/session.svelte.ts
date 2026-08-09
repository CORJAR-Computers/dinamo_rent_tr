// session.svelte.ts — Store de sesión (Svelte 5 runes)
// Persiste el token en localStorage (mismo rol que SessionManager en Rust).

import { authApi, type LoginResult, type SessionData } from '$lib/api';

const TOKEN_KEY = 'dinamo.session.token';
const USER_KEY = 'dinamo.session.user';

interface StoredUser {
	username: string;
	nombre: string | null;
	rol: string | null;
}

class SessionStore {
	// Runes reactivas
	token = $state<string | null>(null);
	user = $state<StoredUser | null>(null);
	debeCambiarPassword = $state(false);
	initialized = $state(false);

	constructor() {
		// Cargar del localStorage al iniciar (solo en cliente)
		if (typeof window !== 'undefined') {
			this.token = localStorage.getItem(TOKEN_KEY);
			const raw = localStorage.getItem(USER_KEY);
			if (raw) {
				try {
					this.user = JSON.parse(raw) as StoredUser;
				} catch {
					this.user = null;
				}
			}
			this.initialized = true;
		}
	}

	get isAuthenticated(): boolean {
		return this.token !== null && this.user !== null;
	}

	/** Establece la sesión tras un login exitoso */
	setSession(result: LoginResult): void {
		this.token = result.sessionId;
		this.user = { username: result.username, nombre: result.nombre, rol: result.rol };
		this.debeCambiarPassword = result.debeCambiarPassword;
		if (typeof window !== 'undefined') {
			localStorage.setItem(TOKEN_KEY, result.sessionId);
			localStorage.setItem(USER_KEY, JSON.stringify(this.user));
		}
	}

	/** Valida la sesión contra el backend (guards de rutas) */
	async validate(): Promise<boolean> {
		if (!this.token) return false;
		try {
			const data: SessionData = await authApi.getSession(this.token);
			this.user = { username: data.username, nombre: data.nombre, rol: data.role };
			this.debeCambiarPassword = data.debeCambiarPassword;
			return true;
		} catch {
			this.clear();
			return false;
		}
	}

	async logout(): Promise<void> {
		if (this.token) {
			try {
				await authApi.logout(this.token);
			} catch {
				// Ignorar errores de logout
			}
		}
		this.clear();
	}

	clear(): void {
		this.token = null;
		this.user = null;
		this.debeCambiarPassword = false;
		if (typeof window !== 'undefined') {
			localStorage.removeItem(TOKEN_KEY);
			localStorage.removeItem(USER_KEY);
		}
	}

	/** ¿El usuario tiene uno de los roles requeridos? */
	hasRole(roles: string[]): boolean {
		if (!this.user?.rol) return false;
		return roles.includes(this.user.rol);
	}
}

export const session = new SessionStore();

/**
 * Helper de un solo uso para obtener el sessionId activo.
 *
 * Reemplaza el patrón `const sid = () => session.token ?? ''` repetido en
 * 15 rutas. Importar desde el store:
 *
 * ```ts
 * import { sid } from '$lib/stores/session.svelte';
 * // …
 * autoApi.listar(sid(), …);
 * ```
 *
 * Si no hay sesión activa devuelve '' (las APIs lanzarán ApiError de
 * "no autenticado" en el backend; el guard de ruta redirige a /login).
 */
export function sid(): string {
	return session.token ?? '';
}
