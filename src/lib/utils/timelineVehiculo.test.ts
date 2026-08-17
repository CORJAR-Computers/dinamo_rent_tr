// src/lib/utils/timelineVehiculo.test.ts — Tests del cruce rentas↔multas
// por vehículo (builder puro, sin Tauri).

import { describe, expect, it } from 'vitest';
import type { Comparendo, Renta } from '$lib/api';
import { construirTimelineVehiculo } from './timelineVehiculo';

function renta(overrides: Partial<Renta> = {}): Renta {
	return {
		id: 1,
		noContrato: 42,
		anioContrato: 2026,
		placa: 'ABC123',
		idCliente: 7,
		nombreCliente: 'Ana Martínez',
		noLicencia: null,
		nacionalidad: null,
		fechaRecogida: '2026-07-01',
		horaRecogida: '09:00',
		ubicacionRecogida: null,
		fechaRetorno: '2026-07-10',
		horaRetorno: '18:00',
		ubicacionRetorno: null,
		diasCalculados: 9,
		horasExtras: 0,
		valorDia: '150000.00',
		valorHoraExtra: '10000.00',
		valorDiaExtra: '0.00',
		costoLavado: '0.00',
		costoSilla: '0.00',
		costoRetorno: '0.00',
		costoDomicilio: '0.00',
		costoCables: '0.00',
		costoInversor: '0.00',
		descuento: '0.00',
		subtotal: '1350000.00',
		impuestos: '0.00',
		cobraIva: true,
		tieneComision: false,
		comision: '0.00',
		valorNeto: '1350000.00',
		total: '1350000.00',
		abono: '500000.00',
		saldoPendiente: '850000.00',
		estado: 'Cerrada',
		observaciones: null,
		fechaDevolucionReal: null,
		horaDevolucionReal: null,
		kmFinal: null,
		tanqueFinal: null,
		kmSalida: '42000',
		tanqueSalida: 'Lleno',
		idReserva: null,
		createdAt: '2026-07-01 09:00:00.000',
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
		fechaInfraccion: '2026-07-05',
		horaInfraccion: '14:30',
		monto: '580000.00',
		numeroComparendo: '250010000000123',
		idRenta: null,
		idCliente: null,
		estado: 'Pendiente',
		observaciones: 'Exceso de velocidad',
		createdAt: null,
		updatedAt: null,
		origen: 'Manual',
		ultimoVistoSimit: null,
		responsable: null,
		...overrides
	};
}

describe('construirTimelineVehiculo', () => {
	it('cruza la multa con la renta del día (responsable.idRenta)', () => {
		const r = renta();
		const m = comparendo({ responsable: { idRenta: 1, nombreCliente: 'Ana Martínez', noContrato: 42, anioContrato: 2026, fechaRecogida: '2026-07-01', fechaRetorno: '2026-07-10', estadoRenta: 'Cerrada' } });

		const tl = construirTimelineVehiculo([r], [m]);

		expect(tl.rentas).toHaveLength(1);
		expect(tl.rentas[0].multas).toEqual([m]);
		expect(tl.totalMultas).toBe(1);
		// La multa también aparece como evento propio
		expect(tl.eventos.filter((e) => e.tipo === 'multa')).toHaveLength(1);
	});

	it('usa la devolución real como fin de la renta si existe', () => {
		const r = renta({ fechaRetorno: '2026-07-10', fechaDevolucionReal: '2026-07-08' });

		const tl = construirTimelineVehiculo([r], []);

		expect(tl.rentas[0].fin).toBe('2026-07-08');
	});

	it('una multa sin responsable no se asocia a ninguna renta', () => {
		const r = renta({ fechaRecogida: '2026-06-01', fechaRetorno: '2026-06-10' });
		const m = comparendo({ fechaInfraccion: '2026-07-05', responsable: null });

		const tl = construirTimelineVehiculo([r], [m]);

		expect(tl.rentas[0].multas).toHaveLength(0);
		expect(tl.multas).toHaveLength(1);
		expect(tl.eventos.some((e) => e.tipo === 'multa')).toBe(true);
	});

	it('suma el pendiente solo de las multas Pendiente', () => {
		const m1 = comparendo({ monto: '580000.00', estado: 'Pendiente' });
		const m2 = comparendo({ id: 2, monto: '320000.00', estado: 'Pagado' });

		const tl = construirTimelineVehiculo([], [m1, m2]);

		expect(tl.totalPendiente).toBe(580000);
	});

	it('ordena los eventos por fecha (rentas y multas mezcladas)', () => {
		const r1 = renta({ id: 1, fechaRecogida: '2026-07-01', fechaRetorno: '2026-07-10' });
		const m1 = comparendo({ id: 1, fechaInfraccion: '2026-07-03' });
		const r2 = renta({ id: 2, fechaRecogida: '2026-07-15', fechaRetorno: '2026-07-20' });
		const m2 = comparendo({ id: 2, fechaInfraccion: '2026-07-25' });

		const tl = construirTimelineVehiculo([r2, r1], [m2, m1]);

		expect(tl.eventos.map((e) => e.fecha)).toEqual(['2026-07-01', '2026-07-03', '2026-07-15', '2026-07-25']);
	});
});
