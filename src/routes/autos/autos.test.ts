// src/routes/autos/autos.test.ts — Tests de la página de Autos (flota)
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Auto, AutoDatos, BusinessLists } from '$lib/api';
import AutosPage from './+page.svelte';

function auto(overrides: Partial<Auto> = {}): Auto {
	return {
		placa: 'ABC123',
		marca: 'Toyota',
		modelo: 'Corolla',
		version: 'XEI 1.8',
		color: 'Blanco',
		tipo: 'Automóvil',
		cilindraje: '1800 cc',
		transmision: 'Automática',
		combustible: 'Gasolina',
		noMotor: null,
		noChasis: null,
		propietario: null,
		estado: 'Disponible',
		costoFijoMensual: '1500000',
		kilometraje: 12345,
		ubicacion: null,
		tipoAdquisicion: 'Propio',
		proximoAceite: null,
		proximoFrenos: null,
		vencimientoSoat: null,
		vencimientoTecnico: null,
		vencimientoExtintor: null,
		vencimientoBateria: null,
		observaciones: null,
		fechaIngreso: '2026-01-10',
		createdAt: null,
		...overrides
	};
}

const LISTS: BusinessLists = {
	tiposAuto: ['Automóvil', 'Camioneta', 'Van', 'Lujo', 'Moto'],
	tiposTransmision: ['Automática', 'Mecánica'],
	tiposCombustible: ['Gasolina', 'Diesel', 'Híbrido', 'Eléctrico', 'Gas'],
	estadosAuto: ['Disponible', 'Rentado', 'Mantenimiento', 'Vendido', 'Baja'],
	tiposAdquisicion: ['Propio', 'Leasing', 'Subarrendado'],
	tiposDoc: [],
	estadosCliente: [],
	estadosReserva: [],
	tiposGasto: [],
	nivelTanque: [],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesConEliminar: ['Administrador', 'Supervisor'],
	rolesDisponibles: []
};

function setSesion(rol = 'Administrador') {
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: 'admin',
		nombre: 'Administrador',
		rol,
		debeCambiarPassword: false
	});
}

beforeEach(() => {
	session.clear();
	// El guard de sesión exige sesión activa para cargar la página
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
	tauri.register('alertas_autos', () => []);
});

describe('página de Autos', () => {
	it('lista los vehículos de la flota', async () => {
		tauri.register('listar_autos', () => [
			auto({ placa: 'ABC123', marca: 'Toyota', modelo: 'Corolla' }),
			auto({ placa: 'XYZ987', marca: 'Mazda', modelo: 'CX-5', estado: 'Rentado' })
		]);

		render(AutosPage);

		expect(await screen.findByText('ABC123')).toBeInTheDocument();
		expect(screen.getByText('XYZ987')).toBeInTheDocument();
		expect(screen.getByText('Toyota')).toBeInTheDocument();
		expect(screen.getByText('Mazda')).toBeInTheDocument();
		// 2 vehículos en el encabezado
		expect(screen.getByText(/2 vehículos/)).toBeInTheDocument();
	});

	it('muestra el estado vacío cuando no hay vehículos', async () => {
		tauri.register('listar_autos', () => []);

		render(AutosPage);

		expect(await screen.findByText('No hay vehículos')).toBeInTheDocument();
		expect(screen.getByText(/0 vehículos/)).toBeInTheDocument();
	});

	it('crea un vehículo desde el modal', async () => {
		tauri.register('listar_autos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: AutoDatos }) =>
			auto({ placa: 'ABC123', marca: 'Toyota', modelo: 'Corolla' })
		);
		tauri.register('crear_auto', crear);

		render(AutosPage);
		await screen.findByText('No hay vehículos');

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Auto' }));
		expect(await screen.findByRole('dialog')).toBeInTheDocument();

		await fireEvent.input(screen.getByPlaceholderText('ABC123'), { target: { value: 'abc123' } });
		await fireEvent.input(screen.getByPlaceholderText('Ej: Toyota'), { target: { value: 'Toyota' } });
		await fireEvent.input(screen.getByPlaceholderText('Ej: Corolla'), { target: { value: 'Corolla' } });

		await fireEvent.click(screen.getByRole('button', { name: 'Crear vehículo' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		// El handler de invoke recibe el objeto de args: { sessionId, datos }
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: AutoDatos };
		expect(args.datos.placa).toBe('abc123');
		expect(args.datos.marca).toBe('Toyota');
		expect(args.datos.modelo).toBe('Corolla');
		// El modal se cierra tras guardar
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida campos obligatorios antes de guardar', async () => {
		tauri.register('listar_autos', () => []);
		const crear = vi.fn(() => auto());
		tauri.register('crear_auto', crear);

		render(AutosPage);
		await screen.findByText('No hay vehículos');

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Auto' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Crear vehículo' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('La placa, marca y modelo son obligatorios.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('edita un vehículo existente', async () => {
		tauri.register('listar_autos', () => [auto({ placa: 'ABC123' })]);
		const actualizar = vi.fn((_args: { sessionId: string; placa: string; datos: AutoDatos }) =>
			auto({ placa: 'ABC123', estado: 'Mantenimiento' })
		);
		tauri.register('actualizar_auto', actualizar);

		render(AutosPage);
		await screen.findByText('ABC123');

		await fireEvent.click(screen.getByTitle('Editar'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Editar vehículo ABC123');

		// FormField usa <span> como label (sin `for`), así que localizamos el
		// select de estado por su valor actual (único: 'Disponible')
		const select = screen.getByDisplayValue('Disponible');
		await fireEvent.change(select, { target: { value: 'Mantenimiento' } });

		await fireEvent.click(screen.getByRole('button', { name: 'Guardar cambios' }));

		await waitFor(() => expect(actualizar).toHaveBeenCalledTimes(1));
		const args = actualizar.mock.calls[0][0] as { sessionId: string; placa: string; datos: AutoDatos };
		expect(args.placa).toBe('ABC123');
		expect(args.datos.estado).toBe('Mantenimiento');
	});

	it('elimina un vehículo tras confirmar', async () => {
		tauri.register('listar_autos', () => [auto({ placa: 'ABC123' })]);
		const eliminar = vi.fn((_args: { sessionId: string; placa: string }) => undefined);
		tauri.register('eliminar_auto', eliminar);

		render(AutosPage);
		await screen.findByText('ABC123');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar vehículo');
		expect(screen.getByText(/eliminar el vehículo ABC123/i)).toBeInTheDocument();

		// Botón del diálogo (hay otro "Eliminar" con title en la fila)
		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; placa: string };
		expect(args.placa).toBe('ABC123');
	});

	it('oculta el botón Eliminar para el rol Operador', async () => {
		setSesion('Operador');
		tauri.register('listar_autos', () => [auto({ placa: 'ABC123' })]);

		render(AutosPage);
		await screen.findByText('ABC123');

		expect(screen.queryByTitle('Eliminar')).not.toBeInTheDocument();
	});

	it('muestra el botón Eliminar para el rol Supervisor', async () => {
		setSesion('Supervisor');
		tauri.register('listar_autos', () => [auto({ placa: 'ABC123' })]);

		render(AutosPage);
		await screen.findByText('ABC123');

		expect(screen.getByTitle('Eliminar')).toBeInTheDocument();
	});

	it('filtra por búsqueda con debounce', async () => {
		const listar = vi.fn((_args: { sessionId: string; busqueda: string | null }) => [auto({ placa: 'ABC123' })]);
		tauri.register('listar_autos', listar);

		render(AutosPage);
		await screen.findByText('ABC123');
		expect(listar).toHaveBeenCalledTimes(1);

		await fireEvent.input(screen.getByPlaceholderText('Buscar por placa, marca o modelo...'), {
			target: { value: 'corolla' }
		});

		// Debounce de 350 ms → segunda llamada con el término
		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; busqueda: string | null };
		expect(args.busqueda).toBe('corolla');
	});

	it('formatea el kilometraje en es-CO', async () => {
		tauri.register('listar_autos', () => [auto({ kilometraje: 1234567 })]);

		render(AutosPage);

		await screen.findByText('ABC123');
		expect(screen.getByText('1.234.567 km')).toBeInTheDocument();
	});

	it('abre el panel de copiado con Ctrl+Shift+C (modal abierto en crear)', async () => {
		tauri.register('listar_autos', () => [auto({ placa: 'EGN754' })]);

		render(AutosPage);
		await screen.findByText('EGN754');

		// Con el modal cerrado el atajo no hace nada
		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		expect(screen.queryByText(/Copiar datos de un vehículo existente/)).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('button', { name: 'Nuevo Auto' }));
		await screen.findByRole('dialog');

		const btnPanel = screen.getByRole('button', { name: /Copiar datos de un vehículo existente/ });
		expect(btnPanel).toHaveAttribute('aria-expanded', 'false');

		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		await waitFor(() => expect(btnPanel).toHaveAttribute('aria-expanded', 'true'));

		// El buscador del panel queda enfocado para escribir directo
		const search = screen.getByPlaceholderText('Buscar por placa, marca o modelo…');
		await waitFor(() => expect(search).toHaveFocus());

		// Segunda pulsación: toggle cierra el panel
		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		await waitFor(() => expect(btnPanel).toHaveAttribute('aria-expanded', 'false'));
	});

	it('no abre el panel de copiado con Ctrl+Shift+C en modo edición', async () => {
		tauri.register('listar_autos', () => [auto({ placa: 'ABC123' })]);

		render(AutosPage);
		await screen.findByText('ABC123');

		await fireEvent.click(screen.getByTitle('Editar'));
		await screen.findByRole('dialog');

		fireEvent.keyDown(window, { key: 'c', ctrlKey: true, shiftKey: true });
		expect(screen.queryByText(/Copiar datos de un vehículo existente/)).not.toBeInTheDocument();
	});
});
