// src/routes/gastos/gastos.test.ts — Tests de la página de Gastos (caja menor)
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Gasto, GastoDatos, TotalesGastos, Auto, BusinessLists } from '$lib/api';
import GastosPage from './+page.svelte';

function gasto(overrides: Partial<Gasto> = {}): Gasto {
	return {
		id: 1,
		placa: 'ABC123',
		fecha: '2026-08-01',
		categoria: 'Combustible',
		descripcion: 'Tanqueo vehículo',
		monto: '120000.00',
		comprobante: 'F-0001',
		usuario: 'admin',
		createdAt: null,
		updatedAt: null,
		...overrides
	};
}

function auto(placa: string, marca = 'Toyota', modelo = 'Corolla'): Auto {
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
		kilometraje: 0,
		ubicacion: null,
		tipoAdquisicion: null,
		proximoAceite: null,
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

function totales(overrides: Partial<TotalesGastos> = {}): TotalesGastos {
	return {
		totalGeneral: '250000.00',
		totalMes: '80000.00',
		porPlaca: [{ clave: 'ABC123', total: '150000.00' }],
		porCategoria: [{ clave: 'Combustible', total: '200000.00' }],
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
	tiposGasto: ['Combustible', 'Peajes', 'Lavado', 'Mantenimiento', 'Repuestos', 'Parqueadero', 'Seguros', 'Multas', 'Papelería', 'Otros'],
	nivelTanque: [],
	tiposMantenimiento: [],
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
	tauri.register('listar_autos', () => [auto('ABC123'), auto('XYZ987', 'Mazda', 'CX-5')]);
	tauri.register('totales_gastos', () => totales());
});

describe('página de Gastos', () => {
	it('lista los gastos registrados con totales', async () => {
		tauri.register('listar_gastos', () => [
			gasto({ id: 1, placa: 'ABC123', descripcion: 'Tanqueo vehículo' }),
			gasto({ id: 2, placa: 'XYZ987', categoria: 'Peajes', monto: '45000.00', descripcion: 'Peaje Barranquilla' })
		]);

		render(GastosPage);

		expect(await screen.findByText('Tanqueo vehículo')).toBeInTheDocument();
		expect(screen.getByText('Peaje Barranquilla')).toBeInTheDocument();
		// Resumen de totales
		expect(screen.getByText(/Total general/i)).toBeInTheDocument();
		expect(screen.getByText(/Por placa/i)).toBeInTheDocument();
		expect(screen.getByText(/Por categoría/i)).toBeInTheDocument();
		// 2 gastos en el encabezado
		expect(screen.getByText(/2 gastos registrados/)).toBeInTheDocument();
	});

	it('muestra el estado vacío cuando no hay gastos', async () => {
		tauri.register('listar_gastos', () => []);

		render(GastosPage);

		expect(await screen.findByText('No hay gastos')).toBeInTheDocument();
		expect(screen.getByText(/0 gastos registrados/)).toBeInTheDocument();
	});

	it('crea un gasto desde el modal', async () => {
		tauri.register('listar_gastos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: GastoDatos }) => gasto({ id: 9 }));
		tauri.register('crear_gasto', crear);

		render(GastosPage);
		await screen.findByText('No hay gastos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Gasto' }));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toBeInTheDocument();

		// Fecha por defecto = hoy (no la tocamos); completamos el resto.
		// FormField usa <span> como label (sin `for`), así que localizamos el
		// select de categoría por su valor actual (único: 'Selecciona...')
		await fireEvent.change(within(dialogo).getByDisplayValue('Selecciona...'), {
			target: { value: 'Combustible' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Ej: Cambio de aceite 15W-40'), {
			target: { value: 'Tanqueo' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Ej: 120000'), {
			target: { value: '80000' }
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar gasto' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: GastoDatos };
		expect(args.datos.categoria).toBe('Combustible');
		expect(args.datos.descripcion).toBe('Tanqueo');
		expect(args.datos.monto).toBe('80000');
		// El modal se cierra tras guardar
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida los campos obligatorios antes de guardar', async () => {
		tauri.register('listar_gastos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: GastoDatos }) => gasto());
		tauri.register('crear_gasto', crear);

		render(GastosPage);
		await screen.findByText('No hay gastos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Gasto' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Registrar gasto' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('La categoría es obligatoria.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('edita un gasto existente', async () => {
		tauri.register('listar_gastos', () => [gasto({ id: 7, descripcion: 'Tanqueo vehículo', monto: '120000.00' })]);
		const actualizar = vi.fn((_args: { sessionId: string; id: number; datos: GastoDatos }) =>
			gasto({ id: 7, monto: '130000.00' })
		);
		tauri.register('actualizar_gasto', actualizar);

		render(GastosPage);
		await screen.findByText('Tanqueo vehículo');

		await fireEvent.click(screen.getByTitle('Editar'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Editar gasto #7');

		// El monto actual viene como '120000.00' en el input
		const montoInput = screen.getByDisplayValue('120000.00');
		await fireEvent.input(montoInput, { target: { value: '130000' } });

		await fireEvent.click(screen.getByRole('button', { name: 'Guardar cambios' }));

		await waitFor(() => expect(actualizar).toHaveBeenCalledTimes(1));
		const args = actualizar.mock.calls[0][0] as { sessionId: string; id: number; datos: GastoDatos };
		expect(args.id).toBe(7);
		expect(args.datos.monto).toBe('130000');
	});

	it('elimina un gasto tras confirmar', async () => {
		tauri.register('listar_gastos', () => [gasto({ id: 3, descripcion: 'Tanqueo vehículo' })]);
		const eliminar = vi.fn((_args: { sessionId: string; id: number }) => undefined);
		tauri.register('eliminar_gasto', eliminar);

		render(GastosPage);
		await screen.findByText('Tanqueo vehículo');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar gasto');
		expect(screen.getByText(/eliminar el gasto «Tanqueo vehículo»/i)).toBeInTheDocument();

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(3);
	});

	it('oculta el botón Eliminar para el rol Operador', async () => {
		setSesion('Operador');
		tauri.register('listar_gastos', () => [gasto()]);

		render(GastosPage);
		await screen.findByText('Tanqueo vehículo');

		expect(screen.queryByTitle('Eliminar')).not.toBeInTheDocument();
	});

	it('muestra el botón Eliminar para el rol Supervisor', async () => {
		setSesion('Supervisor');
		tauri.register('listar_gastos', () => [gasto()]);

		render(GastosPage);
		await screen.findByText('Tanqueo vehículo');

		expect(screen.getByTitle('Eliminar')).toBeInTheDocument();
	});

	it('filtra por placa con el selector', async () => {
		const listar = vi.fn((_args: { sessionId: string; placa: string | null; categoria: string | null }) => [gasto()]);
		tauri.register('listar_gastos', listar);

		render(GastosPage);
		await screen.findByText('Tanqueo vehículo');
		expect(listar).toHaveBeenCalledTimes(1);

		// El selector de placa tiene aria-label "Filtrar por placa"
		const select = screen.getByLabelText('Filtrar por placa');
		await fireEvent.change(select, { target: { value: 'XYZ987' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; placa: string | null };
		expect(args.placa).toBe('XYZ987');
	});
});
