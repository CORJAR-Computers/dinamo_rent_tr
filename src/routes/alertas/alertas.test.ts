// src/routes/alertas/alertas.test.ts — Tests de la página de Alertas
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { AlertaVencimiento, AlertaKm, Renta, Comparendo } from '$lib/api';
import AlertasPage from './+page.svelte';

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

function alerta(overrides: Partial<AlertaVencimiento> = {}): AlertaVencimiento {
	return {
		placa: 'ABC123',
		marca: 'Toyota',
		modelo: 'Corolla',
		tipo: 'SOAT',
		fecha: '2026-08-20',
		diasRestantes: 13,
		detalle: 'SOAT vence pronto',
		critica: false,
		...overrides
	};
}

function alertaKm(overrides: Partial<AlertaKm> = {}): AlertaKm {
	return {
		placa: 'XYZ987',
		marca: 'Mazda',
		modelo: 'CX-5',
		tipo: 'Cambio de aceite',
		kmActual: 48000,
		kmProximo: 50000,
		kmRestante: 2000,
		critica: false,
		...overrides
	};
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
		fechaRecogida: '2026-08-01',
		horaRecogida: null,
		ubicacionRecogida: null,
		fechaRetorno: '2026-08-04',
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
		responsable: null,
		...overrides
	};
}

beforeEach(() => {
	session.clear();
	setSesion();
});

describe('página de Alertas', () => {
	it('consolida vencimientos, km, rentas y comparendos', async () => {
		// Una alerta de SOAT vencida (crítica) y una normal
		tauri.register('alertas_autos', () => [
			alerta({ tipo: 'SOAT', diasRestantes: -2, critica: true }),
			alerta({ placa: 'XYZ987', tipo: 'Batería', diasRestantes: 10 })
		]);
		tauri.register('alertas_km_mantenimiento', () => [alertaKm()]);
		tauri.register('listar_rentas', () => [renta()]);
		tauri.register('listar_comparendos', () => [comparendo()]);

		render(AlertasPage);

		expect(await screen.findByText('Vencimientos de vehículos (2)')).toBeInTheDocument();
		expect(screen.getAllByText(/SOAT/).length).toBeGreaterThan(0);
		expect(screen.getByText('Vencido hace 2 días')).toBeInTheDocument();
		expect(screen.getByText('Mantenimiento por kilometraje (1)')).toBeInTheDocument();
		expect(screen.getByText(/Cambio de aceite/)).toBeInTheDocument();
		expect(screen.getByText('Rentas por vencer (1)')).toBeInTheDocument();
		expect(screen.getByText('Comparendos pendientes (1)')).toBeInTheDocument();
		// Resumen
		expect(screen.getByText('Vencimientos de vehículos')).toBeInTheDocument();
		expect(screen.getByText('Comparendos pendientes')).toBeInTheDocument();
	});

	it('muestra estados vacíos cuando no hay alertas', async () => {
		tauri.register('alertas_autos', () => []);
		tauri.register('alertas_km_mantenimiento', () => []);
		tauri.register('listar_rentas', () => []);
		tauri.register('listar_comparendos', () => []);

		render(AlertasPage);

		// Los ✅ ahora son iconos SVG sin texto (los estados vacíos terminan en «.»)
		expect(await screen.findByText('Sin vencimientos próximos.')).toBeInTheDocument();
		expect(screen.getByText('Sin mantenimientos próximos por km.')).toBeInTheDocument();
		expect(screen.getByText(/No hay rentas activas por vencer/)).toBeInTheDocument();
		expect(screen.getByText('Sin comparendos pendientes de pago.')).toBeInTheDocument();
	});

	it('filtra solo alertas críticas', async () => {
		tauri.register('alertas_autos', () => [
			alerta({ tipo: 'SOAT', diasRestantes: -2, critica: true }),
			alerta({ placa: 'XYZ987', tipo: 'Batería', diasRestantes: 10, critica: false })
		]);
		tauri.register('alertas_km_mantenimiento', () => []);
		tauri.register('listar_rentas', () => []);
		tauri.register('listar_comparendos', () => []);

		render(AlertasPage);
		await screen.findByText('Vencimientos de vehículos (2)');

		await fireEvent.click(screen.getByRole('checkbox', { name: /Solo críticas/i }));

		expect(screen.getByText('Vencimientos de vehículos (1)')).toBeInTheDocument();
		expect(screen.queryByText(/Batería/)).not.toBeInTheDocument();
		expect(screen.getAllByText(/SOAT/).length).toBeGreaterThan(0);
	});

	it('refresca los datos con el botón', async () => {
		const alertas = vi.fn(() => [alerta()]);
		tauri.register('alertas_autos', alertas);
		tauri.register('alertas_km_mantenimiento', () => []);
		tauri.register('listar_rentas', () => []);
		tauri.register('listar_comparendos', () => []);

		render(AlertasPage);
		await screen.findByText('Vencimientos de vehículos (1)');
		expect(alertas).toHaveBeenCalledTimes(1);

		await fireEvent.click(screen.getByRole('button', { name: /Refrescar/i }));

		await waitFor(() => expect(alertas).toHaveBeenCalledTimes(2));
	});
});
