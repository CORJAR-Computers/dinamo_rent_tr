// src/routes/informes/informes.test.ts — Tests de la página de Informes
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { goto } from '$app/navigation';
import { session } from '$lib/stores/session.svelte';
import type { InformeMensual, BusinessLists } from '$lib/api';
import InformesPage from './+page.svelte';

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
	rolesConInformes: ['Administrador', 'Supervisor'],
	rolesConUsuarios: ['Administrador'],
	rolesConEliminar: ['Administrador', 'Supervisor'],
	rolesDisponibles: ['Administrador', 'Supervisor', 'Operador']
};

function informe(overrides: Partial<InformeMensual> = {}): InformeMensual {
	return {
		fechaInicio: '2026-08-01',
		fechaFin: '2026-08-31',
		ingresosPagos: '1200000.00',
		ingresosReservas: '300000.00',
		totalIngresos: '1500000.00',
		egresosGastos: '400000.00',
		egresosMantenimiento: '200000.00',
		egresosComparendos: '100000.00',
		totalEgresos: '700000.00',
		balance: '800000.00',
		gastosPorCategoria: [
			['Combustible', '250000.00'],
			['Lavado', '150000.00']
		],
		rentas: [
			{
				id: 1,
				placa: 'ABC123',
				nombreCliente: 'Cliente Prueba',
				total: '535500.00',
				estado: 'Cerrada',
				fechaRecogida: '2026-08-01'
			}
		],
		utilidadPorVehiculo: [
			{
				placa: 'ABC123',
				vehiculo: 'Toyota Corolla',
				ingresos: '1200000.00',
				costos: '350000.00',
				utilidad: '850000.00'
			},
			{
				placa: 'XYZ987',
				vehiculo: 'Mazda CX-5',
				ingresos: '200000.00',
				costos: '500000.00',
				utilidad: '-300000.00'
			}
		],
		...overrides
	};
}

function setSesion(rol = 'Administrador') {
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: rol === 'Administrador' ? 'admin' : 'usuario',
		nombre: 'Usuario de prueba',
		rol,
		debeCambiarPassword: false
	});
}

beforeEach(() => {
	session.clear();
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
});

describe('página de Informes', () => {
	it('muestra el balance mensual con ingresos, egresos y rentas', async () => {
		tauri.register('informe_mensual', (args?: { sessionId: string; fechaInicio: string; fechaFin: string }) => informe());

		render(InformesPage);

		expect(await screen.findByText('Ingresos del mes')).toBeInTheDocument();
		expect(screen.getAllByText((c) => c.includes('1.500.000')).length).toBeGreaterThan(0);
		expect(screen.getAllByText((c) => c.includes('700.000')).length).toBeGreaterThan(0);
		expect(screen.getAllByText((c) => c.includes('800.000')).length).toBeGreaterThan(0);
		// Desglose de gastos por categoría
		expect(screen.getByText('Combustible')).toBeInTheDocument();
		expect(screen.getByText('Lavado')).toBeInTheDocument();
		// Rentas del mes
		expect(screen.getByText('Rentas del mes (1)')).toBeInTheDocument();
		expect(screen.getByText('Cliente Prueba')).toBeInTheDocument();
	});

	it('muestra la utilidad por vehículo con rentables y en pérdida', async () => {
		tauri.register('informe_mensual', () => informe());

		render(InformesPage);

		expect(await screen.findByText('Utilidad por vehículo (2)')).toBeInTheDocument();
		// La placa ABC123 aparece también en la tabla de rentas del mes
		expect(screen.getAllByText('ABC123').length).toBeGreaterThan(0);
		expect(screen.getByText('Toyota Corolla')).toBeInTheDocument();
		expect(screen.getByText('Mazda CX-5')).toBeInTheDocument();
		// Resumen: 1 rentable, 1 en pérdida
		expect(screen.getByText(/1 rentable · 1 en pérdida/)).toBeInTheDocument();
	});

	it('muestra el estado vacío de utilidad cuando no hay movimiento', async () => {
		tauri.register('informe_mensual', () => informe({ utilidadPorVehiculo: [] }));

		render(InformesPage);

		expect(await screen.findByText('Utilidad por vehículo (0)')).toBeInTheDocument();
		expect(screen.getByText('Sin movimiento por vehículo este mes.')).toBeInTheDocument();
	});

	it('llama al backend con las fechas seleccionadas', async () => {
		const mensual = vi.fn((_args: { sessionId: string; fechaInicio: string; fechaFin: string }) => informe());
		tauri.register('informe_mensual', mensual);

		render(InformesPage);

		const inputInicio = screen.getByLabelText('Fecha inicio');
		
		// Esperar a que el backend resuelva las llamadas iniciales
		await waitFor(() => {
			expect(mensual.mock.calls.length).toBeGreaterThanOrEqual(1);
		});
		mensual.mockClear();

		await fireEvent.input(inputInicio, { target: { value: '2026-07-01' } });
		await fireEvent.change(inputInicio, { target: { value: '2026-07-01' } });
		
		await waitFor(() => expect(mensual).toHaveBeenCalledTimes(1));
		const args = mensual.mock.calls[0][0] as { sessionId: string; fechaInicio: string; fechaFin: string };
		expect(args.fechaInicio).toBe('2026-07-01');
	});

	it('muestra el botón de exportar a Excel', async () => {
		tauri.register('informe_mensual', () => informe());

		render(InformesPage);
		await screen.findByText('Ingresos del mes');

		const btn = screen.getByRole('button', { name: /Exportar Excel/i });
		expect(btn).toBeInTheDocument();
	});

	it('muestra error si el backend falla', async () => {
		tauri.register('informe_mensual', () => {
			throw new Error('boom');
		});

		render(InformesPage);

		expect(await screen.findByText(/No se pudo calcular el informe/)).toBeInTheDocument();
	});
});

describe('guard de rol de la página de Informes (roles_con_informes)', () => {
	it('redirige a /dashboard cuando el usuario no tiene rol de informes', async () => {
		setSesion('Operador');
		const mensual = vi.fn(() => informe());
		tauri.register('informe_mensual', mensual);

		render(InformesPage);

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true }));
		// El Operador no debe disparar NINGUNA llamada al informe
		expect(mensual).not.toHaveBeenCalled();
	});

	it('respeta rolesConInformes personalizado de config.ini (no el fallback)', async () => {
		// Config.ini con roles_con_informes = Supervisor: ni el Administrador entra
		setSesion('Administrador');
		tauri.register('get_business_lists', () => ({ ...LISTS, rolesConInformes: ['Supervisor'] }));
		const mensual = vi.fn(() => informe());
		tauri.register('informe_mensual', mensual);

		render(InformesPage);

		await waitFor(() => expect(goto).toHaveBeenCalledWith('/dashboard', { replaceState: true }));
		expect(mensual).not.toHaveBeenCalled();
	});

	it('permite al Supervisor ver el balance con la configuración por defecto', async () => {
		setSesion('Supervisor');
		tauri.register('informe_mensual', () => informe());

		render(InformesPage);

		expect(await screen.findByText('Ingresos del mes')).toBeInTheDocument();
		expect(goto).not.toHaveBeenCalled();
	});
});
