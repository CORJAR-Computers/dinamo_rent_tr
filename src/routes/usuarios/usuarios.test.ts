// src/routes/usuarios/usuarios.test.ts — Tests del guard de rol de la página de Usuarios
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import type { Usuario, BusinessLists } from '$lib/api';
import UsuariosPage from './+page.svelte';

function usuario(overrides: Partial<Usuario> = {}): Usuario {
	return {
		id: 1,
		username: 'admin',
		nombre: 'Administrador',
		rol: 'Administrador',
		email: null,
		activo: true,
		debeCambiarPassword: false,
		intentosFallidos: 0,
		ultimoAcceso: null,
		createdAt: null,
		...overrides
	};
}

const LISTS: BusinessLists = {
	tiposAuto: [],
	tiposTransmision: [],
	tiposCombustible: [],
	estadosAuto: [],
	tiposAdquisicion: [],
	tiposDoc: [],
	estadosCliente: [],
	estadosReserva: [],
	tiposGasto: [],
	nivelTanque: [],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: ['Administrador'],
	rolesConEliminar: [],
	rolesDisponibles: ['Administrador', 'Supervisor', 'Operador']
};	beforeEach(() => {
		session.clear();
		tauri.register('get_business_lists', () => LISTS);
	});

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

describe('guard de rol de la página de Usuarios', () => {
	it('redirige a /dashboard cuando el usuario no es administrador', async () => {
		setSesion('Operador');
		const listar = vi.fn(() => [usuario()]);
		const listas = vi.fn(() => LISTS);
		tauri.register('listar_usuarios', listar);
		tauri.register('get_business_lists', listas);

		render(UsuariosPage);

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true }));
		// El no-admin no debe disparar NINGUNA llamada a la API
		expect(listar).not.toHaveBeenCalled();
		expect(listas).not.toHaveBeenCalled();
	});

	it('redirige a /dashboard para el rol Supervisor', async () => {
		setSesion('Supervisor');
		tauri.register('listar_usuarios', () => [usuario()]);

		render(UsuariosPage);

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true }));
	});

	it('redirige a /login sin sesión (guard de sesión antes que el de rol)', async () => {
		const listar = vi.fn(() => [usuario()]);
		tauri.register('listar_usuarios', listar);

		render(UsuariosPage);

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/login', { replaceState: true }));
		expect(listar).not.toHaveBeenCalled();
	});

	it('muestra la página y carga los usuarios cuando el rol es Administrador', async () => {
		setSesion('Administrador');
		const listar = vi.fn(() => [
			usuario({ id: 1, username: 'admin', rol: 'Administrador' }),
			usuario({ id: 2, username: 'jperez', nombre: 'Juan Pérez', rol: 'Operador' })
		]);
		tauri.register('listar_usuarios', listar);

		render(UsuariosPage);

		expect(await screen.findByText('jperez')).toBeInTheDocument();
		expect(screen.getByText('Juan Pérez')).toBeInTheDocument();
		expect(goto).not.toHaveBeenCalled();
		expect(listar).toHaveBeenCalledTimes(1);
	});
});
