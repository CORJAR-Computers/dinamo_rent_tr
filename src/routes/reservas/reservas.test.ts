// src/routes/reservas/reservas.test.ts — Tests de la página de Reservas
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
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
});
