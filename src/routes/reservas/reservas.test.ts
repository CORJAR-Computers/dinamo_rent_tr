// src/routes/reservas/reservas.test.ts — Tests de la página de Reservas
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import { goto } from '$app/navigation';
import type { Reserva, Auto, Cliente, BusinessLists } from '$lib/api';
import ReservasPage from './+page.svelte';

function reserva(overrides: Partial<Reserva> = {}): Reserva {
	return {
		id: 1,
		idCliente: 1,
		nombreCliente: 'Juan Perez',
		nacionalidad: 'Colombiana',
		categoriaVehiculo: 'Toyota Corolla',
		placaAsignada: 'ABC123',
		fechaRecogida: '2026-08-10',
		horaRecogida: '10:00',
		ubicacionRecogida: 'Aeropuerto',
		fechaRetorno: '2026-08-12',
		horaRetorno: '10:00',
		ubicacionRetorno: 'Oficina',
		diasCalculados: 2,
		horasExtras: 0,
		valorDia: '150000.00',
		valorHoraAdic: '10000.00',
		costoLavado: '0',
		abono: '100000.00',
		total: '300000.00',
		observaciones: null,
		estado: 'Confirmada',
		createdAt: '2026-08-01',
		updatedAt: null,
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
	estadosReserva: ['Confirmada', 'Pendiente', 'Cancelada', 'Completada'],
	tiposGasto: [],
	nivelTanque: [],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesConEliminar: ['Administrador', 'Supervisor'],
	rolesDisponibles: [],
	impuestoPorcentaje: 19
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
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
	tauri.register('listar_autos', () => []);
	tauri.register('listar_clientes', () => []);
	tauri.register('reservas_proximas', () => []);
});

describe('página de Reservas', () => {
	it('lista las reservas con su estado', async () => {
		tauri.register('listar_reservas', () => [
			reserva(),
			reserva({ id: 2, nombreCliente: 'Maria Perez', placaAsignada: 'XYZ987', estado: 'Cancelada' })
		]);

		render(ReservasPage);

		expect(await screen.findByText('Juan Perez')).toBeInTheDocument();
		expect(screen.getAllByText('Confirmada').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Cancelada').length).toBeGreaterThan(0);
		expect(screen.getByText(/2 reservas/i)).toBeInTheDocument();
	});

	it('muestra estado vacío cuando no hay reservas', async () => {
		tauri.register('listar_reservas', () => []);

		render(ReservasPage);

		expect(await screen.findByText(/No hay reservas/i)).toBeInTheDocument();
	});

	it('oculta el botón Eliminar para el rol Operador', async () => {
		setSesion('Operador');
		tauri.register('listar_reservas', () => [reserva()]);

		render(ReservasPage);
		await screen.findByText('Juan Perez');

		expect(screen.queryByTitle('Eliminar')).not.toBeInTheDocument();
	});

	it('muestra el botón Eliminar para el rol Supervisor', async () => {
		setSesion('Supervisor');
		tauri.register('listar_reservas', () => [reserva()]);

		render(ReservasPage);
		await screen.findByText('Juan Perez');

		expect(screen.getByTitle('Eliminar')).toBeInTheDocument();
	});

	it('«Crear renta» navega a /rentas con el id de la reserva', async () => {
		tauri.register('listar_reservas', () => [reserva()]);

		render(ReservasPage);
		await screen.findByText('Juan Perez');

		await fireEvent.click(screen.getByTitle(/Crear renta desde esta reserva/));

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/rentas?desdeReserva=1'));
	});

	it('no muestra «Crear renta» para reservas canceladas o completadas', async () => {
		tauri.register('listar_reservas', () => [
			reserva({ id: 2, nombreCliente: 'Maria Perez', estado: 'Cancelada' }),
			reserva({ id: 3, nombreCliente: 'Luis Diaz', estado: 'Completada' })
		]);

		render(ReservasPage);
		await screen.findByText('Maria Perez');

		expect(screen.queryByTitle(/Crear renta desde esta reserva/)).not.toBeInTheDocument();
	});

	it('muestra botón "Por asignar" cuando la reserva no tiene placa asignada y abre modal de asignación', async () => {
		tauri.register('listar_reservas', () => [
			reserva({
				id: 5,
				nombreCliente: 'Carlos Ruiz',
				categoriaVehiculo: 'Auto Económico',
				placaAsignada: null
			})
		]);
		tauri.register('listar_autos', () => [
			{
				placa: 'XYZ123',
				marca: 'Kia',
				modelo: 'Picanto',
				tipo: 'Auto Económico',
				transmision: 'Automática',
				estado: 'Disponible',
				kilometraje: 15000
			}
		]);

		render(ReservasPage);
		await screen.findByText('Carlos Ruiz');

		const botonPorAsignar = screen.getByTitle(/Sin placa asignada. Clic para asignar vehículo/i);
		expect(botonPorAsignar).toBeInTheDocument();
		expect(botonPorAsignar).toHaveTextContent(/Por asignar/i);

		await fireEvent.click(botonPorAsignar);
		expect(await screen.findByText(/Asignar vehículo a Reserva #5/i)).toBeInTheDocument();
	});

	it('permite asignar vehículo por placa o categoría y guarda con asignar_vehiculo_reserva', async () => {
		tauri.register('listar_reservas', () => [
			reserva({
				id: 8,
				nombreCliente: 'Ana Gomez',
				categoriaVehiculo: 'Auto Económico',
				placaAsignada: null
			})
		]);
		let asignarLlamado = false;
		tauri.register('asignar_vehiculo_reserva', (payload: any) => {
			asignarLlamado = true;
			expect(payload.id).toBe(8);
			expect(payload.categoriaVehiculo).toBe('Auto Automático');
			return reserva({ id: 8, categoriaVehiculo: 'Auto Automático' });
		});

		render(ReservasPage);
		await screen.findByText('Ana Gomez');

		const botonAsignar = screen.getByTitle(/Asignar o cambiar vehículo/i);
		await fireEvent.click(botonAsignar);

		expect(await screen.findByText(/Asignar vehículo a Reserva #8/i)).toBeInTheDocument();

		// Cambiar categoría rápida con chip
		const chipAutomatico = screen.getByRole('button', { name: 'Auto Automático' });
		await fireEvent.click(chipAutomatico);

		const botonGuardar = screen.getByRole('button', { name: /Guardar asignación/i });
		await fireEvent.click(botonGuardar);

		await waitFor(() => expect(asignarLlamado).toBe(true));
	});
});
