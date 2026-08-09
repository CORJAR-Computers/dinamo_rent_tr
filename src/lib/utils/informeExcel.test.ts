// src/lib/utils/informeExcel.test.ts — Tests de la exportación .xlsx del informe
import { describe, it, expect } from 'vitest';
import * as XLSX from 'xlsx';
import type { InformeMensual } from '$lib/api';
import { filasInformeExcel, construirLibroInforme } from './informeExcel';

function informe(overrides: Partial<InformeMensual> = {}): InformeMensual {
	return {
		fechaInicio: '2026-08-01',
		fechaFin: '2026-08-08',
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
			}
		],
		...overrides
	};
}

/** Devuelve el valor plano de una celda (desenvuelve estilo) */
function valor(celda: unknown): string | number {
	return typeof celda === 'object' && celda !== null ? (celda as { v: string | number }).v : (celda as string | number);
}

/** Devuelve el estilo de una celda, si lo tiene */
function estilo(celda: unknown): Record<string, unknown> | undefined {
	return typeof celda === 'object' && celda !== null ? (celda as { estilo?: Record<string, unknown> }).estilo : undefined;
}

describe('filasInformeExcel', () => {
	it('incluye título, periodo y las cuatro secciones', () => {
		const filas = filasInformeExcel(informe(), '2026-08-01 al 2026-08-08');

		expect(String(valor(filas[0][0]))).toContain('INFORME FINANCIERO');
		expect(String(valor(filas[1][0]))).toContain('2026-08-01 al 2026-08-08');
		expect(filas.some((f) => String(valor(f[0])).startsWith('RESUMEN DEL PERIODO'))).toBe(true);
		expect(filas.some((f) => String(valor(f[0])).startsWith('GASTOS POR CATEGORÍA'))).toBe(true);
		expect(filas.some((f) => String(valor(f[0])).startsWith('RENTAS DEL PERIODO'))).toBe(true);
		expect(filas.some((f) => String(valor(f[0])).startsWith('UTILIDAD POR VEHÍCULO'))).toBe(true);
	});

	it('convierte los montos a números y marca la columna como moneda', () => {
		const filas = filasInformeExcel(informe(), 'periodo');

		const balance = filas.find((f) => String(valor(f[0])).startsWith('BALANCE'));
		expect(balance).toBeDefined();
		expect(valor(balance![1])).toBe(800000);
		expect(estilo(balance![1])?.monto).toBe(true);

		// Utilidad por vehículo como número
		const util = filas.find((f) => String(valor(f[0])) === 'ABC123' && String(valor(f[1])).includes('Toyota'));
		expect(util).toBeDefined();
		expect(valor(util![4])).toBe(850000);
	});

	it('incluye los datos de rentas y utilidad en su sección', () => {
		const filas = filasInformeExcel(informe(), 'periodo');
		const texto = filas.map((f) => f.map(valor).join('|')).join('\n');

		expect(texto).toContain('Cliente Prueba');
		expect(texto).toContain('ABC123');
		expect(texto).toContain('Cerrada');
		expect(texto).toContain('Toyota Corolla');
	});

	it('no rompe con secciones vacías', () => {
		const filas = filasInformeExcel(
			informe({ gastosPorCategoria: [], rentas: [], utilidadPorVehiculo: [] }),
			'periodo'
		);

		expect(filas.length).toBeGreaterThan(0);
		expect(filas.some((f) => String(valor(f[0])).startsWith('RENTAS DEL PERIODO (0)'))).toBe(true);
	});
});

describe('construirLibroInforme', () => {
	it('genera un Workbook con la hoja "Informe", anchos y fusiones', () => {
		const wb = construirLibroInforme(informe(), 'periodo');

		expect(wb.SheetNames).toEqual(['Informe']);
		const ws = wb.Sheets['Informe'];
		expect(ws).toBeDefined();
		expect(Array.isArray(ws['!cols'])).toBe(true);
		// Título + periodo fusionados + encabezados de sección
		expect(ws['!merges']?.length ?? 0).toBeGreaterThanOrEqual(6);
		// Título estilizado (fuente blanca sobre azul)
		const titulo = ws['A1'];
		expect(titulo.v).toContain('INFORME FINANCIERO');
		expect(titulo.s?.font?.bold).toBe(true);
	});

	it('el workbook es serializable a archivo y se puede releer', () => {
		const wb = construirLibroInforme(informe(), 'periodo');
		const datos = XLSX.write(wb, { type: 'buffer', bookType: 'xlsx' });
		expect(datos.length).toBeGreaterThan(0);

		const releido = XLSX.read(datos, { type: 'buffer' });
		const ws = releido.Sheets[releido.SheetNames[0]];
		expect(String(ws['A1'].v)).toContain('INFORME FINANCIERO');
		// Balance sigue siendo numérico tras el round-trip
		const filaBalance = Object.keys(ws)
			.filter((k) => /^A\d+$/.test(k) && typeof ws[k].v === 'string' && String(ws[k].v).startsWith('BALANCE'))
			.map((k) => ws[k]);
		expect(filaBalance.length).toBe(1);
	});
});
