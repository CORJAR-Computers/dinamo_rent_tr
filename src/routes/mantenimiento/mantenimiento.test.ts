// src/routes/mantenimiento/mantenimiento.test.ts — Tests de la página de Mantenimiento
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Mantenimiento, MantenimientoDatos, TotalesMantenimiento, Auto, BusinessLists } from '$lib/api';
import MantenimientoPage from './+page.svelte';

function mantenimiento(overrides: Partial<Mantenimiento> = {}): Mantenimiento {
	return {
		id: 1,
		placa: 'ABC123',
		vehiculo: 'Toyota Corolla',
		tipo: 'CAMBIO ACEITE',
		fecha: '2026-08-01',
		descripcion: 'Cambio de aceite 15W-40',
		observaciones: null,
		costo: '200000.00',
		kmProximoCambioAceite: 50000,
		total: '200000.00',
		createdAt: null,
		updatedAt: null,
		...overrides
	};
}

function auto(placa: string, marca = 'Toyota', modelo = 'Corolla', proximoAceite: number | null = null): Auto {
	return {
		placa,
		marca,
		modelo,
		version: null,
		color: null,
		tipo: 'Automóvil',
		cilindraje: null,
		transmision: null,
		combustible: null,
		noMotor: null,
		noChasis: null,
		propietario: null,
		estado: 'Disponible',
		costoFijoMensual: '1500000',
		kilometraje: 42000,
		ubicacion: null,
		tipoAdquisicion: null,
		proximoAceite,
		proximoFrenos: null,
		vencimientoSoat: null,
		vencimientoTecnico: null,
		vencimientoExtintor: null,
		vencimientoBateria: null,
		observaciones: null,
		fechaIngreso: '2026-01-10',
		createdAt: null
	};
}

function totales(overrides: Partial<TotalesMantenimiento> = {}): TotalesMantenimiento {
	return {
		totalGeneral: '350000.00',
		porPlaca: [{ clave: 'ABC123', total: '350000.00' }],			porTipo: [{ clave: 'CAMBIO ACEITE', total: '200000.00' }],
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
	tiposMantenimiento: ['Cambio Aceite', 'Frenos', 'Llantas', 'Batería', 'Tecno-Mecánica', 'Lavado General', 'Reparación Mecánica', 'Otro'],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesConEliminar: ['Administrador', 'Supervisor'],
	rolesDisponibles: []
,
	impuestoPorcentaje: 19,
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
	tauri.register('listar_autos', () => [auto('ABC123', 'Toyota', 'Corolla', 50000), auto('XYZ987', 'Mazda', 'CX-5')]);
	tauri.register('totales_mantenimiento', () => totales());
	tauri.register('alertas_km_mantenimiento', () => []);
});

describe('página de Mantenimiento', () => {
	it('lista el historial de mantenimientos con totales', async () => {
		tauri.register('listar_mantenimientos', () => [
			mantenimiento({ id: 1, placa: 'ABC123', tipo: 'Cambio Aceite', costo: '200000.00' }),
			mantenimiento({ id: 2, placa: 'XYZ987', tipo: 'FRENOS', costo: '150000.00', descripcion: 'Cambio de pastillas' })
		]);

		render(MantenimientoPage);

		expect(await screen.findByText('Cambio de aceite 15W-40')).toBeInTheDocument();
		expect(screen.getByText('Cambio de pastillas')).toBeInTheDocument();
		expect(screen.getByText(/Total invertido/i)).toBeInTheDocument();
		expect(screen.getByText(/Por placa/i)).toBeInTheDocument();
		expect(screen.getByText(/Por tipo/i)).toBeInTheDocument();
		expect(screen.getByText(/2 registros de mantenimiento/)).toBeInTheDocument();
	});

	it('muestra el estado vacío cuando no hay mantenimientos', async () => {
		tauri.register('listar_mantenimientos', () => []);

		render(MantenimientoPage);

		expect(await screen.findByText('No hay mantenimientos')).toBeInTheDocument();
		expect(screen.getByText(/0 registros de mantenimiento/)).toBeInTheDocument();
	});

	it('muestra las alertas por kilometraje', async () => {
		tauri.register('listar_mantenimientos', () => []);
		tauri.register('alertas_km_mantenimiento', () => [
			{
				placa: 'ABC123',
				marca: 'Toyota',
				modelo: 'Corolla',
				tipo: 'Cambio de aceite',
				kmActual: 50000,
				kmProximo: 50000,
				kmRestante: 0,
				critica: true
			}
		]);

		render(MantenimientoPage);

		expect(await screen.findByText('Alertas por kilometraje')).toBeInTheDocument();
		expect(screen.getByText(/vencido · km 50.000 > 50.000 km/i)).toBeInTheDocument();
	});

	it('crea un mantenimiento desde el modal', async () => {
		tauri.register('listar_mantenimientos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: MantenimientoDatos }) =>
			mantenimiento({ id: 9 })
		);
		tauri.register('crear_mantenimiento', crear);

		render(MantenimientoPage);
		await screen.findByText('No hay mantenimientos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Mantenimiento' }));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toBeInTheDocument();

		// Vehículo: combobox con búsqueda (escribir placa + Enter). Tipo: select.
		const placaCombo = within(dialogo).getByPlaceholderText('Buscar placa, marca o modelo…');
		await fireEvent.focus(placaCombo);
		await fireEvent.input(placaCombo, { target: { value: 'ABC123' } });
		await fireEvent.keyDown(placaCombo, { key: 'Enter' });
		const tipoSelect = within(dialogo).getByDisplayValue('Selecciona...');
		await fireEvent.change(tipoSelect, { target: { value: 'FRENOS' } });
		await fireEvent.input(screen.getByPlaceholderText('Ej: 350000'), {
			target: { value: '150000' }
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar mantenimiento' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: MantenimientoDatos };
		expect(args.datos.placa).toBe('ABC123');
		expect(args.datos.tipo).toBe('FRENOS');
		expect(args.datos.costo).toBe('150000');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida los campos obligatorios antes de guardar', async () => {
		tauri.register('listar_mantenimientos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: MantenimientoDatos }) =>
			mantenimiento()
		);
		tauri.register('crear_mantenimiento', crear);

		render(MantenimientoPage);
		await screen.findByText('No hay mantenimientos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Mantenimiento' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Registrar mantenimiento' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('La placa es obligatoria.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('edita un mantenimiento existente', async () => {
		tauri.register('listar_mantenimientos', () => [
			mantenimiento({ id: 7, tipo: 'FRENOS', costo: '150000.00' })
		]);
		const actualizar = vi.fn(
			(_args: { sessionId: string; id: number; datos: MantenimientoDatos }) =>
				mantenimiento({ id: 7, costo: '160000.00' })
		);
		tauri.register('actualizar_mantenimiento', actualizar);

		render(MantenimientoPage);
		await screen.findByText('Cambio de aceite 15W-40');

		await fireEvent.click(screen.getByTitle('Editar'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Editar mantenimiento #7');

		const costoInput = screen.getByDisplayValue('150000.00');
		await fireEvent.input(costoInput, { target: { value: '160000' } });

		await fireEvent.click(screen.getByRole('button', { name: 'Guardar cambios' }));

		await waitFor(() => expect(actualizar).toHaveBeenCalledTimes(1));
		const args = actualizar.mock.calls[0][0] as {
			sessionId: string;
			id: number;
			datos: MantenimientoDatos;
		};
		expect(args.id).toBe(7);
		expect(args.datos.costo).toBe('160000');
	});

	it('elimina un mantenimiento tras confirmar', async () => {
		tauri.register('listar_mantenimientos', () => [
			mantenimiento({ id: 3, tipo: 'CAMBIO ACEITE' })
		]);
		const eliminar = vi.fn((_args: { sessionId: string; id: number }) => undefined);
		tauri.register('eliminar_mantenimiento', eliminar);

		render(MantenimientoPage);
		await screen.findByText('Cambio de aceite 15W-40');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar mantenimiento');
		expect(screen.getByText(/eliminar el mantenimiento de tipo «CAMBIO ACEITE»/i)).toBeInTheDocument();

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(3);
	});

	it('oculta el botón Eliminar para el rol Operador', async () => {
		setSesion('Operador');
		tauri.register('listar_mantenimientos', () => [mantenimiento()]);

		render(MantenimientoPage);
		await screen.findByText('Cambio de aceite 15W-40');

		expect(screen.queryByTitle('Eliminar')).not.toBeInTheDocument();
	});

	it('muestra el botón Eliminar para el rol Supervisor', async () => {
		setSesion('Supervisor');
		tauri.register('listar_mantenimientos', () => [mantenimiento()]);

		render(MantenimientoPage);
		await screen.findByText('Cambio de aceite 15W-40');

		expect(screen.getByTitle('Eliminar')).toBeInTheDocument();
	});

	it('filtra por placa con el selector', async () => {
		const listar = vi.fn(
			(_args: { sessionId: string; placa: string | null; tipo: string | null }) =>
				[mantenimiento()]
		);
		tauri.register('listar_mantenimientos', listar);

		render(MantenimientoPage);
		await screen.findByText('Cambio de aceite 15W-40');
		expect(listar).toHaveBeenCalledTimes(1);

		const select = screen.getByLabelText('Filtrar por placa');
		await fireEvent.change(select, { target: { value: 'XYZ987' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; placa: string | null };
		expect(args.placa).toBe('XYZ987');
	});
});
