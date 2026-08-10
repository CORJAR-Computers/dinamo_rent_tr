// src/routes/comparendos/comparendos.test.ts — Tests de la página de Comparendos
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Comparendo, ComparendoDatos, Auto, BusinessLists } from '$lib/api';
import ComparendosPage from './+page.svelte';

function comparendo(overrides: Partial<Comparendo> = {}): Comparendo {
	return {
		id: 1,
		placa: 'ABC123',
		vehiculo: 'Toyota Corolla',
		fechaInfraccion: '2026-08-01',
		horaInfraccion: '14:30',
		monto: '580000.00',
		numeroComparendo: null,
		idRenta: null,
		idCliente: null,
		estado: 'Pendiente',
		observaciones: 'Exceso de velocidad',
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
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
	tauri.register('listar_autos', () => [auto('ABC123'), auto('XYZ987', 'Mazda', 'CX-5')]);
});

describe('página de Comparendos', () => {
	it('lista los comparendos con su estado', async () => {
		tauri.register('listar_comparendos', () => [
			comparendo(),
			comparendo({ id: 2, placa: 'XYZ987', monto: '320000.00', estado: 'Pagado', observaciones: 'Foto-detección' })
		]);

		render(ComparendosPage);

		expect(await screen.findByText('Exceso de velocidad')).toBeInTheDocument();
		expect(screen.getByText('Foto-detección')).toBeInTheDocument();
		expect(screen.getAllByText((c) => c.includes('580.000')).length).toBeGreaterThan(0);
		expect(screen.getAllByText((c) => c.includes('320.000')).length).toBeGreaterThan(0);
		expect(screen.getAllByText('Pendiente').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Pagado').length).toBeGreaterThan(0);
		expect(screen.getByText(/2 comparendos/)).toBeInTheDocument();
	});

	it('muestra el estado vacío cuando no hay comparendos', async () => {
		tauri.register('listar_comparendos', () => []);

		render(ComparendosPage);

		expect(await screen.findByText('No hay comparendos')).toBeInTheDocument();
		expect(screen.getByText(/0 comparendos/)).toBeInTheDocument();
	});

	it('registra un comparendo desde el modal', async () => {
		tauri.register('listar_comparendos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: ComparendoDatos }) => comparendo({ id: 9 }));
		tauri.register('crear_comparendo', crear);

		render(ComparendosPage);
		await screen.findByText('No hay comparendos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Comparendo' }));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toBeInTheDocument();

		await fireEvent.change(within(dialogo).getByDisplayValue('— Seleccionar vehículo —'), {
			target: { value: 'ABC123' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Ej: 580000'), {
			target: { value: '580000' }
		});
		await fireEvent.input(screen.getByPlaceholderText('HH:MM'), {
			target: { value: '14:30' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Ej: Exceso de velocidad, foto-detección...'), {
			target: { value: 'Exceso de velocidad' }
		});

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Registrar comparendo' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: ComparendoDatos };
		expect(args.datos.placa).toBe('ABC123');
		expect(args.datos.monto).toBe('580000');
		expect(args.datos.estado).toBe('Pendiente');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida los campos obligatorios antes de guardar', async () => {
		tauri.register('listar_comparendos', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: ComparendoDatos }) => comparendo());
		tauri.register('crear_comparendo', crear);

		render(ComparendosPage);
		await screen.findByText('No hay comparendos');

		await fireEvent.click(screen.getByRole('button', { name: 'Registrar Comparendo' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Registrar comparendo' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('La placa es obligatoria.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('marca un comparendo como pagado tras confirmar', async () => {
		tauri.register('listar_comparendos', () => [comparendo({ id: 3 })]);
		const pagar = vi.fn((_args: { sessionId: string; id: number }) =>
			comparendo({ id: 3, estado: 'Pagado' })
		);
		tauri.register('marcar_pagado_comparendo', pagar);

		render(ComparendosPage);
		await screen.findByText('Exceso de velocidad');

		await fireEvent.click(screen.getByTitle('Marcar como pagado'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Marcar comparendo como pagado');

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Marcar pagado' }));

		await waitFor(() => expect(pagar).toHaveBeenCalledTimes(1));
		const args = pagar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(3);
	});

	it('edita un comparendo existente', async () => {
		tauri.register('listar_comparendos', () => [comparendo({ id: 7 })]);
		const actualizar = vi.fn((_args: { sessionId: string; id: number; datos: ComparendoDatos }) =>
			comparendo({ id: 7, monto: '650000.00' })
		);
		tauri.register('actualizar_comparendo', actualizar);

		render(ComparendosPage);
		await screen.findByText('Exceso de velocidad');

		await fireEvent.click(screen.getByTitle('Editar'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Editar comparendo #7');

		const montoInput = screen.getByDisplayValue('580000.00');
		await fireEvent.input(montoInput, { target: { value: '650000' } });

		await fireEvent.click(screen.getByRole('button', { name: 'Guardar cambios' }));

		await waitFor(() => expect(actualizar).toHaveBeenCalledTimes(1));
		const args = actualizar.mock.calls[0][0] as { sessionId: string; id: number; datos: ComparendoDatos };
		expect(args.id).toBe(7);
		expect(args.datos.monto).toBe('650000');
	});

	it('elimina un comparendo tras confirmar', async () => {
		tauri.register('listar_comparendos', () => [comparendo({ id: 3 })]);
		const eliminar = vi.fn((_args: { sessionId: string; id: number }) => undefined);
		tauri.register('eliminar_comparendo', eliminar);

		render(ComparendosPage);
		await screen.findByText('Exceso de velocidad');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar comparendo');

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(3);
	});

	it('filtra por estado con el selector', async () => {
		const listar = vi.fn((_args: { sessionId: string; estado: string | null; placa: string | null }) => [comparendo()]);
		tauri.register('listar_comparendos', listar);

		render(ComparendosPage);
		await screen.findByText('Exceso de velocidad');
		expect(listar).toHaveBeenCalledTimes(1);

		const select = screen.getByLabelText('Filtrar por estado');
		await fireEvent.change(select, { target: { value: 'Pagado' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; estado: string | null };
		expect(args.estado).toBe('Pagado');
	});
});
