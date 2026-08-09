// src/routes/cambiar-password/load.test.ts — Tests del guard en el load
import { describe, it, expect, beforeEach } from 'vitest';
import { session } from '$lib/stores/session.svelte';
import { load } from './+page';

function setSesion(debeCambiarPassword: boolean) {
	session.setSession({
		success: true,
		sessionId: 'tok-cp',
		username: 'jperez',
		nombre: 'Juan Pérez',
		rol: 'Operador',
		debeCambiarPassword
	});
}

/** Captura el redirect que lanza load() y devuelve su destino */
function destinoDe(fn: () => unknown): string {
	try {
		fn();
	} catch (e) {
		if (e instanceof Error && (e as unknown as { location?: string }).location) {
			return (e as unknown as { location: string }).location;
		}
		throw e;
	}
	throw new Error('load() no lanzó redirect');
}

beforeEach(() => {
	session.clear();
});

describe('load de /cambiar-password', () => {
	it('redirige a /login sin sesión', () => {
		expect(destinoDe(() => load())).toBe('/login');
	});

	it('redirige a /dashboard cuando la sesión no exige el cambio', () => {
		setSesion(false);
		expect(destinoDe(() => load())).toBe('/dashboard');
	});

	it('permite renderizar cuando la sesión exige el cambio', () => {
		setSesion(true);
		expect(load()).toEqual({});
	});
});
