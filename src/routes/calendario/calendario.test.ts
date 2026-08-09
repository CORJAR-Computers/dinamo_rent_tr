// src/routes/calendario/calendario.test.ts — Tests de la página de Calendario
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Renta, Reserva } from '$lib/api';
import CalendarioPage from './+page.svelte';

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

// Fechas relativas al mes actual para que el grid renderice los chips
function diaDelMesActual(dia: number): string {
	const hoy = new Date();
	return `${hoy.getFullYear()}-${String(hoy.getMonth() + 1).padStart(2, '0')}-${String(dia).padStart(2, '0')}`;
}

function renta(overrides: Partial<Renta> = {}): Renta {
	return {
		id: 1,
		noContrato: 42,
		anioContrato: 2026,
		placa: 'ABC123',
		idCliente: null,
		nombreCliente: 'Cliente Prueba',
		noLicencia: null,
		nacionalidad: null,
		fechaRecogida: diaDelMesActual(1),
		horaRecogida: null,
		ubicacionRecogida: null,
		fechaRetorno: diaDelMesActual(4),
		horaRetorno: null,
		ubicacionRetorno: null,
		diasCalculados: 3,
		horasExtras: 0,
		valorDia: '150000.00',
		valorHoraExtra: '0.00',
		valorDiaExtra: '0.00',
		costoLavado: '0.00',
		costoSilla: '0.00',
		costoRetorno: '0.00',
		costoDomicilio: '0.00',
		costoCables: '0.00',
		costoInversor: '0.00',
		descuento: '0.00',
		subtotal: '450000.00',
		impuestos: '0.00',
		total: '450000.00',
		abono: '0.00',
		saldoPendiente: '450000.00',
		estado: 'Activo',
		observaciones: null,
		fechaDevolucionReal: null,
		horaDevolucionReal: null,
		kmFinal: null,
		tanqueFinal: null,
		kmSalida: '42000',
		tanqueSalida: 'Lleno',
		idReserva: null,
		createdAt: null,
		vehiculo: 'Toyota Corolla',
		pagos: [],
		inspecciones: [],
		...overrides
	};
}

function reserva(overrides: Partial<Reserva> = {}): Reserva {
	return {
		id: 5,
		idCliente: null,
		nombreCliente: 'Reserva Cliente',
		nacionalidad: null,
		categoriaVehiculo: 'Camioneta',
		placaAsignada: 'XYZ987',
		fechaRecogida: diaDelMesActual(2),
		horaRecogida: null,
		ubicacionRecogida: null,
		fechaRetorno: diaDelMesActual(6),
		horaRetorno: null,
		ubicacionRetorno: null,
		diasCalculados: 4,
		horasExtras: 0,
		valorDia: '200000.00',
		valorHoraAdic: '0.00',
		abono: '100000.00',
		total: '800000.00',
		observaciones: null,
		estado: 'Confirmada',
		createdAt: null,
		updatedAt: null,
		...overrides
	};
}

beforeEach(() => {
	session.clear();
	setSesion();
});

describe('página de Calendario', () => {
	it('muestra el mes actual con rentas y reservas en sus días', async () => {
		tauri.register('listar_rentas', () => [renta()]);
		tauri.register('listar_reservas', () => [reserva()]);

		render(CalendarioPage);

		// Título del mes actual
		const ahora = new Date();
		await waitFor(() => {
			expect(
				screen.getByText(ahora.toLocaleDateString('es-CO', { month: 'long', year: 'numeric' }))
			).toBeInTheDocument();
		});

		// Sin solapamientos
		expect(screen.getByText(/0 conflictos de fechas detectados/)).toBeInTheDocument();
		// La renta aparece en el día 1 del mes (chip R1) — esperar la carga async
		expect((await screen.findAllByText(/R1 · Cliente/)).length).toBeGreaterThan(0);
		// La reserva aparece (chip Rv5)
		expect((await screen.findAllByText(/Rv5 · Reserva/)).length).toBeGreaterThan(0);
	});

	it('detecta solapamiento de fechas del mismo vehículo', async () => {
		// Renta y reserva del mismo vehículo con rangos cruzados
		tauri.register('listar_rentas', () => [renta()]);
		tauri.register('listar_reservas', () => [reserva({ placaAsignada: 'ABC123', fechaRecogida: diaDelMesActual(3) })]);

		render(CalendarioPage);

		await waitFor(() => {
			expect(screen.getByText(/1 conflicto de fechas detectado/)).toBeInTheDocument();
		});
	});

	it('abre el detalle del día al hacer clic', async () => {
		tauri.register('listar_rentas', () => [renta()]);
		tauri.register('listar_reservas', () => []);

		render(CalendarioPage);
		// Esperar a que cargue la renta (chip R1) para que el detalle tenga datos
		await screen.findAllByText(/R1 · Cliente/);

		// Clic en la celda del día 1 del mes actual
		const hoy = new Date();
		const dia1 = `${hoy.getFullYear()}-${String(hoy.getMonth() + 1).padStart(2, '0')}-01`;
		const celda = screen.getByRole('button', { name: new RegExp(`Día ${dia1}`) });
		await fireEvent.click(celda);

		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Renta #1');
		expect(dialogo).toHaveTextContent('Cliente Prueba');
	});
});
