// src/routes/cambiar-password/cambiar-password.test.ts — Tests del formulario.
// El guard de la ruta (sesión + debeCambiarPassword) vive en el load (+page.ts)
// y se cubre en load.test.ts; aquí solo se prueba el componente.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import CambiarPasswordPage from './+page.svelte';

beforeEach(() => {
	session.clear();
});

describe('formulario de cambio de contraseña', () => {
	it('muestra los campos del formulario', () => {
		session.setSession({
			success: true,
			sessionId: 'tok-cp',
			username: 'jperez',
			nombre: 'Juan Pérez',
			rol: 'Operador',
			debeCambiarPassword: true
		});

		render(CambiarPasswordPage);

		expect(screen.getByLabelText('Contraseña actual')).toBeInTheDocument();
		expect(screen.getByLabelText('Nueva contraseña')).toBeInTheDocument();
		expect(screen.getByLabelText('Confirmar nueva contraseña')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Guardar contraseña' })).toBeInTheDocument();
	});

	it('valida que los campos estén completos', async () => {
		session.setSession({
			success: true,
			sessionId: 'tok-cp',
			username: 'jperez',
			nombre: 'Juan Pérez',
			rol: 'Operador',
			debeCambiarPassword: true
		});

		render(CambiarPasswordPage);
		await fireEvent.click(screen.getByRole('button', { name: 'Guardar contraseña' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('Todos los campos son obligatorios.');
		});
		expect(goto).not.toHaveBeenCalled();
	});

	it('redirige a /dashboard si el flag se corrige tras el mount (F5 con flag stale)', async () => {
		session.setSession({
			success: true,
			sessionId: 'tok-cp',
			username: 'jperez',
			nombre: 'Juan Pérez',
			rol: 'Operador',
			debeCambiarPassword: true
		});

		render(CambiarPasswordPage);
		expect(screen.getByLabelText('Contraseña actual')).toBeInTheDocument();
		expect(goto).not.toHaveBeenCalled();

		// El layout corrige el flag tras validar contra el backend
		session.debeCambiarPassword = false;

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true }));
	});

	it('redirige a /dashboard al completar el cambio exitoso', async () => {
		session.setSession({
			success: true,
			sessionId: 'tok-cp',
			username: 'jperez',
			nombre: 'Juan Pérez',
			rol: 'Operador',
			debeCambiarPassword: true
		});
		const cambiar = vi.fn((_args: Record<string, unknown>) => undefined);
		tauri.register('change_password', cambiar);

		render(CambiarPasswordPage);

		await fireEvent.input(screen.getByLabelText('Contraseña actual'), {
			target: { value: 'Anterior#1' }
		});
		await fireEvent.input(screen.getByLabelText('Nueva contraseña'), {
			target: { value: 'Nueva#123' }
		});
		await fireEvent.input(screen.getByLabelText('Confirmar nueva contraseña'), {
			target: { value: 'Nueva#123' }
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Guardar contraseña' }));

		await waitFor(() => expect(cambiar).toHaveBeenCalledTimes(1));
		const args = cambiar.mock.calls[0][0] as {
			username: string;
			currentPassword: string;
			newPassword: string;
		};
		expect(args.username).toBe('jperez');
		expect(args.newPassword).toBe('Nueva#123');

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard'));
		expect(session.debeCambiarPassword).toBe(false);
	});
});
