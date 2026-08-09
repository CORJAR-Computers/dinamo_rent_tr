// src/routes/auditoria/auditoria.test.ts — Tests de la página de Auditoría
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import type { AuditoriaEvento, AuditoriaResultado } from '$lib/api';
import AuditoriaPage from './+page.svelte';

function evento(overrides: Partial<AuditoriaEvento> = {}): AuditoriaEvento {
	return {
		id: 1,
		usuario: 'admin',
		accion: 'LOGIN OK',
		mensaje: 'usuario=admin, rol=Administrador',
		ip: 'local',
		fecha: '2026-08-07 15:22:52.9560',
		...overrides
	};
}

function resultado(eventos: AuditoriaEvento[], total = eventos.length, pagina = 1): AuditoriaResultado {
	return { eventos, total, pagina, porPagina: 50 };
}

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
	// El guard de rol exige sesión admin para ver la página
	setSesion('Administrador');
	tauri.register('usuarios_auditoria', () => ['admin', 'd350533700']);
	tauri.register('acciones_auditoria', () => [
		'LOGIN OK',
		'LOGIN FALLIDO',
		'USUARIO CREADO',
		'USUARIO ELIMINADO',
		'CUENTA DESBLOQUEADA'
	]);
});

describe('página de Auditoría', () => {
	it('lista los eventos de auditoría', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) =>
			resultado([
				evento({ id: 1, usuario: 'admin', accion: 'LOGIN OK' }),
				evento({ id: 2, usuario: 'd350533700', accion: 'CUENTA DESBLOQUEADA' })
			])
		);
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);

		expect(await screen.findByText('LOGIN OK')).toBeInTheDocument();
		// 'CUENTA DESBLOQUEADA' también existe como <option> del filtro de acción,
		// así que usamos getAllByText (al menos la fila + la opción)
		expect(screen.getAllByText('CUENTA DESBLOQUEADA').length).toBeGreaterThanOrEqual(1);
		expect(screen.getByText(/2 eventos registrados/)).toBeInTheDocument();
		// Se llamó con los filtros vacíos
		expect(listar).toHaveBeenCalledTimes(1);
	});

	it('muestra el estado vacío sin eventos', async () => {
		tauri.register('listar_auditoria', () => resultado([], 0));

		render(AuditoriaPage);

		expect(await screen.findByText('Sin eventos')).toBeInTheDocument();
		expect(screen.getByText(/0 eventos registrados/)).toBeInTheDocument();
	});

	it('filtra por usuario con el selector', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()], 1));
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);
		await screen.findByText('LOGIN OK');
		expect(listar).toHaveBeenCalledTimes(1);

		const select = screen.getByLabelText('Filtrar por usuario');
		await fireEvent.change(select, { target: { value: 'admin' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { usuario: string };
		expect(args.usuario).toBe('admin');
	});

	it('filtra por acción con el selector', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()], 1));
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);
		await screen.findByText('LOGIN OK');
		expect(listar).toHaveBeenCalledTimes(1);

		const select = screen.getByLabelText('Filtrar por acción');
		await fireEvent.change(select, { target: { value: 'LOGIN FALLIDO' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { accion: string };
		expect(args.accion).toBe('LOGIN FALLIDO');
	});

	it('filtra por rango de fechas', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()], 1));
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);
		await screen.findByText('LOGIN OK');

		// Los inputs de fecha están etiquetados con 'Desde'/'Hasta'. Svelte
		// bind:value en inputs escucha el evento `input`, no `change`.
		const desde = screen.getByLabelText('Desde');
		const hasta = screen.getByLabelText('Hasta');
		await fireEvent.input(desde, { target: { value: '2026-08-01' } });
		await fireEvent.input(hasta, { target: { value: '2026-08-31' } });

		// Ambos cambios ocurren en el mismo tick: Svelte agrupa el $effect en
		// una sola llamada a cargar con los dos filtros (1 inicial + 1 con ambos)
		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { fechaDesde: string; fechaHasta: string };
		expect(args.fechaDesde).toBe('2026-08-01');
		expect(args.fechaHasta).toBe('2026-08-31');
	});

	it('busca por texto libre con debounce', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()], 1));
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);
		await screen.findByText('LOGIN OK');
		expect(listar).toHaveBeenCalledTimes(1);

		await fireEvent.input(screen.getByPlaceholderText('Buscar por usuario, acción o detalle...'), {
			target: { value: 'admin' }
		});

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { busqueda: string };
		expect(args.busqueda).toBe('admin');
	});

	describe('guard de rol', () => {
		it('redirige a /dashboard cuando el usuario no es administrador', async () => {
			setSesion('Operador');
			const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()]));
			tauri.register('listar_auditoria', listar);

			render(AuditoriaPage);

			await waitFor(() =>
				expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true })
			);
			// El no-admin no debe disparar NINGUNA llamada a la API
			expect(listar).not.toHaveBeenCalled();
		});

		it('redirige a /dashboard para el rol Supervisor', async () => {
			setSesion('Supervisor');
			tauri.register('listar_auditoria', () => resultado([evento()]));

			render(AuditoriaPage);

			await waitFor(() =>
				expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true })
			);
		});

		it('redirige a /login sin sesión (guard de sesión antes que el de rol)', async () => {
			session.clear();
			const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()]));
			tauri.register('listar_auditoria', listar);

			render(AuditoriaPage);

			await waitFor(() => expect(goto).toHaveBeenCalledWith('/login', { replaceState: true }));
			expect(listar).not.toHaveBeenCalled();
		});
	});

	it('limpia los filtros con el botón', async () => {
		const listar = vi.fn((_args: Record<string, unknown>) => resultado([evento()], 1));
		tauri.register('listar_auditoria', listar);

		render(AuditoriaPage);
		await screen.findByText('LOGIN OK');

		await fireEvent.change(screen.getByLabelText('Filtrar por usuario'), {
			target: { value: 'admin' }
		});
		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });

		await fireEvent.click(screen.getByRole('button', { name: 'Limpiar filtros' }));
		await waitFor(() => expect(listar).toHaveBeenCalledTimes(3), { timeout: 2000 });
		// El selector vuelve al valor vacío
		expect(screen.getByLabelText('Filtrar por usuario')).toHaveValue('');
	});
});
