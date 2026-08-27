// src/lib/utils/informeExcel.test.ts — Tests de la exportación .xlsx del informe
// Migrado de `xlsx` a `exceljs` (ver informeExcel.ts).
import { describe, it, expect } from 'vitest';
import ExcelJS from 'exceljs';
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
		totalComisiones: '50000.00',
		ingresosNetos: '1450000.00',
		balanceNeto: '750000.00',
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
				comision: '50000.00',
				valorNeto: '485500.00',
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
	return typeof celda === 'object' && celda !== null
		? (celda as { v: string | number }).v
		: (celda as string | number);
}

/** Devuelve el estilo de una celda, si lo tiene */
function estilo(celda: unknown): Record<string, unknown> | undefined {
	return typeof celda === 'object' && celda !== null
		? (celda as { estilo?: Record<string, unknown> }).estilo
		: undefined;
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
		const util = filas.find(
			(f) => String(valor(f[0])) === 'ABC123' && String(valor(f[1])).includes('Toyota')
		);
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

	it('incluye comisiones, ingresos netos y balance neto', () => {
		const filas = filasInformeExcel(informe(), 'periodo');
		const texto = filas.map((f) => f.map(valor).join('|')).join('\n');

		expect(texto).toContain('Comisiones (intermediarios)');
		expect(texto).toContain('Ingresos netos (tras comisiones)');
		expect(texto).toContain('BALANCE NETO (tras comisiones)');
		expect(texto).toContain('Comisión');
		expect(texto).toContain('Valor neto');
		expect(texto).toContain('-50000');
		expect(texto).toContain('485500');
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

		expect(wb.worksheets.map((w) => w.name)).toEqual(['Informe']);
		const ws = wb.getWorksheet('Informe');
		expect(ws).toBeDefined();
		// Anchos de columna asignados (6 columnas)
		expect(ws!.columnCount).toBeGreaterThanOrEqual(6);
		// Título estilizado (fuente bold)
		const titulo = ws!.getCell('A1');
		expect(String(titulo.value)).toContain('INFORME FINANCIERO');
		expect(titulo.font?.bold).toBe(true);
		// Las fusiones se cuentan en el modelo de ExcelJS (mínimo: título A1:F1 +
		// periodo A2:F2 + 4 encabezados de sección = 6).
		const wsModel = (ws as unknown as { model?: { merges?: unknown[] } }).model;
		const mergesCount = wsModel?.merges?.length ?? 0;
		expect(mergesCount).toBeGreaterThanOrEqual(6);
	});

	it('el workbook es serializable a buffer y se puede releer', async () => {
		const wb = construirLibroInforme(informe(), 'periodo');
		const buffer = await wb.xlsx.writeBuffer();
		expect(buffer.byteLength).toBeGreaterThan(0);

		const wb2 = new ExcelJS.Workbook();
		await wb2.xlsx.load(buffer);
		const ws = wb2.getWorksheet(wb2.worksheets[0]!.name)!;
		expect(String(ws.getCell('A1').value ?? '')).toContain('INFORME FINANCIERO');
		// Balance sigue siendo numérico tras el round-trip (lo buscamos en la columna A)
		const balanceRow = Array.from({ length: ws.rowCount }, (_, i) => i + 1)
			.map((r) => ws.getCell(`A${r}`))
			.find((c) => typeof c.value === 'string' && String(c.value).startsWith('BALANCE'));
		expect(balanceRow).toBeDefined();
		// La celda adyacente (B) del balance debe ser numérica
		if (balanceRow) {
			const monto = ws.getCell(`B${balanceRow.row}`);
			expect(
				typeof monto.value === 'number' ||
					(monto.value as { result?: number })?.result !== undefined
			).toBe(true);
		}
	});
});
