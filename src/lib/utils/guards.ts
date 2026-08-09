// src/lib/utils/guards.ts — Guards de sesión y rol reutilizables para páginas.
//
// Centraliza el patrón de protección de rutas:
// - validarSesion(): valida la sesión contra el backend y redirige a /login
//   si no es válida (guard del layout, corre al montar la app).
// - guardSesion(): redirige a /login si no hay sesión (defensa en profundidad
//   en cada página, además del guard del layout, para cortar llamadas a la API
//   antes de que la validación asíncrona del layout termine).
// - guardRole(): redirige a una ruta segura si la sesión no tiene el rol
//   requerido (páginas admin-only).
// Cada función devuelve un booleano que las funciones de carga de datos usan
// para abortar sin disparar llamadas a la API durante la redirección.
//
// Uso típico:
//   onMount(async () => { await validarSesion(); ... });   // layout
//   onMount(() => guardSesion());                          // páginas
//   onMount(() => guardRole(['Administrador'], '/dashboard'));
//   async function cargar() {
//     if (!haySesion()) return; // sin llamadas a la API
//     if (!tieneRol(['Administrador'])) return;
//     ...
//   }

import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';

/**
 * Valida la sesión contra el backend (session.validate). Si no es válida,
 * redirige a /login (reemplazando el historial) y devuelve `false`.
 * Es el guard de sesión del layout: corre al montar la app y sincroniza
 * además el rol y el flag de cambio de contraseña desde el backend.
 */
export async function validarSesion(): Promise<boolean> {
	const ok = await session.validate();
	if (!ok) {
		void goto('/login', { replaceState: true });
		return false;
	}
	return true;
}

/**
 * ¿Hay una sesión activa (token + usuario)?
 * Síncrono y seguro para usarlo en funciones de carga de datos.
 */
export function haySesion(): boolean {
	return session.isAuthenticated;
}

/**
 * Guard de ruta por sesión. Si no hay sesión activa, redirige a `/login`
 * (reemplazando el historial para que «atrás» no regrese a la página
 * protegida) y devuelve `false`. Devuelve `true` cuando hay sesión.
 */
export function guardSesion(): boolean {
	if (session.isAuthenticated) return true;
	void goto('/login', { replaceState: true });
	return false;
}

/**
 * ¿La sesión actual tiene uno de los roles requeridos?
 * Síncrono y seguro para usarlo en funciones de carga de datos.
 */
export function tieneRol(roles: string[]): boolean {
	return session.hasRole(roles);
}

/**
 * Guard de ruta por rol. Si la sesión actual NO tiene uno de los roles
 * requeridos, redirige a `fallback` (reemplazando el historial para que el
 * botón «atrás» no regrese a la página prohibida) y devuelve `false`.
 *
 * Devuelve `true` cuando el usuario sí está autorizado.
 */
export function guardRole(roles: string[], fallback = '/dashboard'): boolean {
	if (session.hasRole(roles)) return true;
	void goto(fallback, { replaceState: true });
	return false;
}
