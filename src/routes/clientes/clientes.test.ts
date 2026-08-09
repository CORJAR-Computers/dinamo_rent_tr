// src/routes/clientes/clientes.test.ts — Tests de la página de Clientes
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Cliente, ClienteConPii, ClienteDatos, BusinessLists } from '$lib/api';
import ClientesPage from './+page.svelte';

function cliente(overrides: Partial<Cliente> = {}): Cliente {
	return {
		id: 1,
		tipoDoc: 'Cédula',
		noDoc: '1036672369',
		nombres: 'Ana',
		apellidos: 'Pérez',
		nombreCompleto: 'Ana Pérez',
		celular: '3101234567',
		celular2: null,
		email: 'ana@correo.com',
		ciudad: 'Barranquilla',
		estadoRegion: null,
		pais: 'Colombia',
		nacionalidad: 'Colombiana',
		dirResidencia: null,
		dirTemporal: null,
		hotel: null,
		habitacion: null,
		noLicencia: null,
		tipoLicencia: null,
		vencimientoLicencia: null,
		estado: 'Activo',
		createdAt: null,
		...overrides
	};
}

function conPii(c: Cliente, piiOculto = false): ClienteConPii {
	return { cliente: c, piiOculto };
}

const LISTS: BusinessLists = {
	tiposAuto: [],
	tiposTransmision: [],
	tiposCombustible: [],
	estadosAuto: [],
	tiposAdquisicion: [],
	tiposDoc: ['Cédula', 'Pasaporte', 'Cédula Extranjería', 'NIT', 'Licencia USA'],
	estadosCliente: ['Activo', 'Inactivo', 'Lista Negra', 'VIP'],
	estadosReserva: [],
	tiposGasto: [],
	nivelTanque: [],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesDisponibles: []
};

function setSesion() {
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: 'admin',
		nombre: 'Administrador',
		rol: 'Administrador',
		debeCambiarPassword: false
	});
}

beforeEach(() => {
	session.clear();
	// El guard de sesión exige sesión activa para cargar la página
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
});

describe('página de Clientes', () => {
	it('lista los clientes registrados', async () => {
		tauri.register('listar_clientes', () => [
			conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez', noDoc: '1036672369' })),
			conPii(cliente({ id: 2, nombreCompleto: 'Luis Gómez', noDoc: '72145678' }))
		]);

		render(ClientesPage);

		expect(await screen.findByText('Ana Pérez')).toBeInTheDocument();
		expect(screen.getByText('Luis Gómez')).toBeInTheDocument();
		expect(screen.getByText('1036672369')).toBeInTheDocument();
		expect(screen.getByText(/2 clientes registrados/)).toBeInTheDocument();
	});

	it('muestra el banner cuando hay PII legacy oculta', async () => {
		tauri.register('listar_clientes', () => [
			conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }), true),
			conPii(cliente({ id: 2, nombreCompleto: 'Luis Gómez' }), false)
		]);

		render(ClientesPage);

		await screen.findByText('Ana Pérez');
		expect(screen.getByText(/datos de contacto cifrados \(Fernet legacy\)/i)).toBeInTheDocument();
	});

	it('crea un cliente desde el modal', async () => {
		tauri.register('listar_clientes', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: ClienteDatos }) =>
			conPii(cliente({ id: 5, nombreCompleto: 'Ana Pérez' }))
		);
		tauri.register('crear_cliente', crear);

		render(ClientesPage);
		await screen.findByText('No hay clientes');

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Cliente' }));
		expect(await screen.findByRole('dialog')).toBeInTheDocument();

		await fireEvent.input(screen.getByPlaceholderText('Nombres del cliente'), {
			target: { value: 'Ana' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Apellidos del cliente'), {
			target: { value: 'Pérez' }
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Crear cliente' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: ClienteDatos };
		expect(args.datos.nombres).toBe('Ana');
		expect(args.datos.apellidos).toBe('Pérez');
		// Modal cerrado tras guardar
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida el nombre obligatorio antes de guardar', async () => {
		tauri.register('listar_clientes', () => []);
		const crear = vi.fn(() => conPii(cliente({ id: 5 })));
		tauri.register('crear_cliente', crear);

		render(ClientesPage);
		await screen.findByText('No hay clientes');

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Cliente' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Crear cliente' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('El nombre del cliente es obligatorio.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('edita un cliente existente', async () => {
		tauri.register('listar_clientes', () => [conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]);
		const actualizar = vi.fn((_args: { sessionId: string; id: number; datos: ClienteDatos }) =>
			conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))
		);
		tauri.register('actualizar_cliente', actualizar);

		render(ClientesPage);
		await screen.findByText('Ana Pérez');

		await fireEvent.click(screen.getByTitle('Editar'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Editar cliente #1');

		await fireEvent.input(screen.getByPlaceholderText('Nombres del cliente'), {
			target: { value: 'Ana María' }
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Guardar cambios' }));

		await waitFor(() => expect(actualizar).toHaveBeenCalledTimes(1));
		const args = actualizar.mock.calls[0][0] as { sessionId: string; id: number; datos: ClienteDatos };
		expect(args.id).toBe(1);
		expect(args.datos.nombres).toBe('Ana María');
	});

	it('elimina un cliente tras confirmar', async () => {
		tauri.register('listar_clientes', () => [conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]);
		const eliminar = vi.fn((_args: { sessionId: string; id: number }) => undefined);
		tauri.register('eliminar_cliente', eliminar);

		render(ClientesPage);
		await screen.findByText('Ana Pérez');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar cliente');
		expect(screen.getByText(/eliminar a Ana Pérez/i)).toBeInTheDocument();

		// Botón del diálogo (hay otro "Eliminar" con title en la fila)
		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(1);
	});

	it('abre el panel de copiado con Ctrl+Shift+C (modal abierto en crear)', async () => {
		tauri.register('listar_clientes', () => [conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]);

		render(ClientesPage);
		await screen.findByText('Ana Pérez');

		// Con el modal cerrado el atajo no hace nada
		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		expect(screen.queryByText(/Copiar datos de un cliente existente/)).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Cliente' }));
		await screen.findByRole('dialog');

		const btnPanel = screen.getByRole('button', { name: /Copiar datos de un cliente existente/ });
		expect(btnPanel).toHaveAttribute('aria-expanded', 'false');

		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		await waitFor(() => expect(btnPanel).toHaveAttribute('aria-expanded', 'true'));

		// El buscador del panel queda enfocado para escribir directo
		const search = screen.getByPlaceholderText('Buscar por nombre, documento o celular…');
		await waitFor(() => expect(search).toHaveFocus());

		// Segunda pulsación: toggle cierra el panel
		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		await waitFor(() => expect(btnPanel).toHaveAttribute('aria-expanded', 'false'));
	});

	it('no abre el panel de copiado con Ctrl+Shift+C en modo edición', async () => {
		tauri.register('listar_clientes', () => [conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]);

		render(ClientesPage);
		await screen.findByText('Ana Pérez');

		await fireEvent.click(screen.getByTitle('Editar'));
		await screen.findByRole('dialog');

		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		expect(screen.queryByText(/Copiar datos de un cliente existente/)).not.toBeInTheDocument();
	});

	it('filtra por búsqueda con debounce', async () => {
		const listar = vi.fn((_args: { sessionId: string; busqueda: string | null }) =>
			[conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]
		);
		tauri.register('listar_clientes', listar);

		render(ClientesPage);
		await screen.findByText('Ana Pérez');
		expect(listar).toHaveBeenCalledTimes(1);

		await fireEvent.input(screen.getByPlaceholderText('Buscar por nombre, documento o celular...'), {
			target: { value: 'ana' }
		});

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; busqueda: string | null };
		expect(args.busqueda).toBe('ana');
	});
});
