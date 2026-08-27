// src/routes/login/login.test.ts — Tests del flujo de autenticación
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import LoginPage from './+page.svelte';

/** Rechaza igual que Tauri: string JSON del payload de error */
function rejectPayload(payload: { kind: string; message: string }) {
	throw JSON.stringify(payload);
}

const OK_STATUS = {
	isLocked: false,
	lockoutRemainingSeconds: 0,
	failedAttempts: 0,
	remainingAttempts: 5
};

beforeEach(() => {
	session.clear();
});

describe('página de login', () => {
	it('muestra los campos de usuario y contraseña', () => {
		render(LoginPage);
		expect(screen.getByLabelText('Usuario')).toBeInTheDocument();
		expect(screen.getByLabelText('Contraseña')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Ingresar' })).toBeInTheDocument();
	});

	it('valida campos vacíos al enviar', async () => {
		render(LoginPage);
		await fireEvent.click(screen.getByRole('button', { name: 'Ingresar' }));
		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('Ingresa tu usuario y contraseña.');
		});
		expect(goto).not.toHaveBeenCalled();
	});

	it('inicia sesión con éxito y navega al dashboard', async () => {
		tauri.register('login', () => ({
			success: true,
			sessionId: 'tok-123',
			username: 'admin',
			nombre: 'Administrador',
			rol: 'Administrador',
			debeCambiarPassword: false
		}));

		render(LoginPage);
		await fireEvent.input(screen.getByLabelText('Usuario'), { target: { value: 'admin' } });
		await fireEvent.input(screen.getByLabelText('Contraseña'), { target: { value: 'secret' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Ingresar' }));

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard'));
		expect(session.token).toBe('tok-123');
		expect(session.user?.username).toBe('admin');
	});

	it('redirige a cambiar-password cuando la cuenta lo exige', async () => {
		tauri.register('login', () => ({
			success: true,
			sessionId: 'tok-cp',
			username: 'admin',
			nombre: 'Administrador',
			rol: 'Administrador',
			debeCambiarPassword: true
		}));

		render(LoginPage);
		await fireEvent.input(screen.getByLabelText('Usuario'), { target: { value: 'admin' } });
		await fireEvent.input(screen.getByLabelText('Contraseña'), { target: { value: 'secret' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Ingresar' }));

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/cambiar-password'));
		expect(session.debeCambiarPassword).toBe(true);
	});

	it('muestra el mensaje de error del backend cuando el login falla', async () => {
		tauri.register('login', () =>
			rejectPayload({ kind: 'auth', message: 'Usuario o contraseña incorrectos.' })
		);
		tauri.register('get_login_status', () => OK_STATUS);

		render(LoginPage);
		await fireEvent.input(screen.getByLabelText('Usuario'), { target: { value: 'admin' } });
		await fireEvent.input(screen.getByLabelText('Contraseña'), { target: { value: 'wrong' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Ingresar' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('Usuario o contraseña incorrectos.');
		});
		expect(goto).not.toHaveBeenCalled();
		expect(session.token).toBeNull();
	});

	it('bloquea el formulario y avisa cuando la cuenta está bloqueada', async () => {
		tauri.register('get_login_status', () => ({
			isLocked: true,
			lockoutRemainingSeconds: 120,
			failedAttempts: 5,
			remainingAttempts: 0
		}));

		render(LoginPage);
		// El $effect consulta el estado al escribir ≥2 caracteres (debounce 400 ms)
		await fireEvent.input(screen.getByLabelText('Usuario'), { target: { value: 'admin' } });

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent(/Cuenta bloqueada/i);
		});
		expect(screen.getByLabelText('Contraseña')).toBeDisabled();
		expect(screen.getByRole('button', { name: 'Ingresar' })).toBeDisabled();
	});

	it('alterna la visibilidad de la contraseña', async () => {
		render(LoginPage);
		const password = screen.getByLabelText('Contraseña');
		expect(password).toHaveAttribute('type', 'password');

		await fireEvent.click(screen.getByRole('button', { name: 'Mostrar contraseña' }));
		expect(screen.getByLabelText('Contraseña')).toHaveAttribute('type', 'text');

		await fireEvent.click(screen.getByRole('button', { name: 'Ocultar contraseña' }));
		expect(screen.getByLabelText('Contraseña')).toHaveAttribute('type', 'password');
	});
});
