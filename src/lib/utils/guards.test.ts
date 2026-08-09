// src/lib/utils/guards.test.ts — Tests del guard de roles centralizado
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import { guardRole, guardSesion, haySesion, tieneRol, validarSesion } from './guards';

function setSesion(rol: string) {
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: 'usuario',
		nombre: 'Usuario de prueba',
		rol,
		debeCambiarPassword: false
	});
}

beforeEach(() => {
	session.clear();
});

describe('tieneRol', () => {
	it('devuelve true con un rol autorizado', () => {
		setSesion('Administrador');
		expect(tieneRol(['Administrador'])).toBe(true);
	});

	it('devuelve false con un rol no autorizado', () => {
		setSesion('Operador');
		expect(tieneRol(['Administrador'])).toBe(false);
	});

	it('devuelve false sin sesión', () => {
		expect(tieneRol(['Administrador'])).toBe(false);
	});
});

describe('validarSesion', () => {
	it('devuelve true y sincroniza la sesión cuando el backend la valida', async () => {
		setSesion('Operador');
		tauri.register('get_session', () => ({
			username: 'usuario',
			nombre: 'Actualizado',
			role: 'Supervisor',
			debeCambiarPassword: false
		}));

		await expect(validarSesion()).resolves.toBe(true);
		expect(goto).not.toHaveBeenCalled();
		expect(session.user?.rol).toBe('Supervisor');
	});

	it('devuelve false y redirige a /login cuando la sesión no es válida', async () => {
		setSesion('Operador');
		tauri.register('get_session', () => {
			throw 'error';
		});

		await expect(validarSesion()).resolves.toBe(false);
		expect(goto).toHaveBeenCalledWith('/login', { replaceState: true });
		expect(session.isAuthenticated).toBe(false);
	});

	it('devuelve false y redirige a /login sin token', async () => {
		await expect(validarSesion()).resolves.toBe(false);
		expect(goto).toHaveBeenCalledWith('/login', { replaceState: true });
	});
});

describe('haySesion', () => {
	it('devuelve true con una sesión activa', () => {
		setSesion('Operador');
		expect(haySesion()).toBe(true);
	});

	it('devuelve false sin sesión', () => {
		expect(haySesion()).toBe(false);
	});
});

describe('guardSesion', () => {
	it('devuelve true y no redirige cuando hay sesión', () => {
		setSesion('Operador');
		expect(guardSesion()).toBe(true);
		expect(goto).not.toHaveBeenCalled();
	});

	it('devuelve false y redirige a /login sin sesión', () => {
		expect(guardSesion()).toBe(false);
		expect(goto).toHaveBeenCalledWith('/login', { replaceState: true });
	});
});

describe('guardRole', () => {
	it('devuelve true y no redirige cuando el usuario tiene el rol', () => {
		setSesion('Administrador');
		expect(guardRole(['Administrador'], '/dashboard')).toBe(true);
		expect(goto).not.toHaveBeenCalled();
	});

	it('devuelve false y redirige al fallback cuando no tiene el rol', () => {
		setSesion('Operador');
		expect(guardRole(['Administrador'], '/dashboard')).toBe(false);
		expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true });
	});

	it('redirige con replaceState (sin historial que permita volver atrás)', () => {
		setSesion('Supervisor');
		guardRole(['Administrador']);
		expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true });
	});
});
