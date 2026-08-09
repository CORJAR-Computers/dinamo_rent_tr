// informeExcel.ts — Exportación .xlsx real (ExcelJS) del informe financiero.
// Funciones puras: `filasInformeExcel` produce las filas (testeable) y
// `construirLibroInforme` las convierte en un Workbook estilizado (título,
// encabezados con color, formato de moneda, anchos de columna y totales).
//
// MIGRACIÓN (G-C2): reemplazado el paquete `xlsx` (SheetJS CE, descontinuado en
// npm + CVE-2023-30533 prototype pollution, CVE-2024-22363 ReDoS) por
// `exceljs` ^4.4.0 (mantenido, sin CVEs conocidos). Las funciones públicas
// `filasInformeExcel` (pura) y `construirLibroInforme` mantienen su contrato
// (esta última ahora devuelve un `ExcelJS.Workbook` en lugar de `XLSX.WorkBook`).
import ExcelJS from 'exceljs';
import type { InformeMensual } from '$lib/api';

/** Formato numérico de moneda aplicado a las celdas de dinero */
const FORMATO_MONTO = '#,##0';

/** Estilo opcional de una celda exportada */
export interface EstiloCelda {
	bold?: boolean;
	/** Color de fondo (hex sin '#') */
	fill?: string;
	/** Color de fuente (hex sin '#') */
	color?: string;
	fontSize?: number;
	/** Aplica formato numérico de moneda */
	monto?: boolean;
}

/** Celda del informe: valor plano o valor con estilo */
export type CeldaInforme = string | number | { v: string | number; estilo?: EstiloCelda };

export type FilaInforme = CeldaInforme[];

/** Ancho de columnas del libro (A–F) */
const ANCHOS_COLUMNAS: number[] = [42, 18, 16, 14, 14, 14];

/** Color institucional (azul marino) para títulos y encabezados */
const AZUL = '1A237E';
const AZUL_CLARO = 'E8EAF6';

/**
 * Títulos de sección → última columna a fusionar (ancho de su tabla).
 * Las secciones se detectan por prefijo para no depender del conteo de filas.
 */
const SECCIONES: [prefijo: string, ultimaCol: number][] = [
	['RESUMEN DEL PERIODO', 1],
	['GASTOS POR CATEGORÍA', 1],
	['RENTAS DEL PERIODO', 5],
	['UTILIDAD POR VEHÍCULO', 4]
];

function aplanar(fila: FilaInforme): (string | number)[] {
	return fila.map((c) => (typeof c === 'object' ? c.v : c));
}

/** Convierte un hex sin '#' ('1A237E') a ARGB con alpha FF ('FF1A237E') para ExcelJS */
function argb(hex: string): string {
	return hex.length === 8 ? hex : `FF${hex}`;
}

/**
 * Construye las filas del libro (paridad con la vista de la página):
 * título + periodo, resumen del balance, gastos por categoría,
 * rentas del rango y utilidad por vehículo.
 */
export function filasInformeExcel(informe: InformeMensual, periodo: string): FilaInforme[] {
	const filas: FilaInforme[] = [];

	// Título y periodo
	filas.push([{ v: 'DINAMO RENT — INFORME FINANCIERO', estilo: { bold: true, fill: AZUL, color: 'FFFFFF', fontSize: 14 } }]);
	filas.push([{ v: `Periodo: ${periodo}`, estilo: { color: '444444' } }]);
	filas.push([{ v: ' ' }]);

	// Resumen del balance
	filas.push([{ v: 'RESUMEN DEL PERIODO', estilo: { bold: true, fill: AZUL_CLARO } }]);
	filas.push([
		{ v: 'Concepto', estilo: { bold: true } },
		{ v: 'Monto', estilo: { bold: true } }
	]);
	filas.push([
		'Ingresos — pagos de rentas',
		{ v: parseFloat(informe.ingresosPagos) || 0, estilo: { monto: true } }
	]);
	filas.push([
		'Ingresos — abonos de reservas',
		{ v: parseFloat(informe.ingresosReservas) || 0, estilo: { monto: true } }
	]);
	filas.push([
		{ v: 'Total ingresos', estilo: { bold: true } },
		{ v: parseFloat(informe.totalIngresos) || 0, estilo: { bold: true, monto: true } }
	]);
	filas.push([
		'Egresos — gastos',
		{ v: parseFloat(informe.egresosGastos) || 0, estilo: { monto: true } }
	]);
	filas.push([
		'Egresos — mantenimiento',
		{ v: parseFloat(informe.egresosMantenimiento) || 0, estilo: { monto: true } }
	]);
	filas.push([
		'Egresos — comparendos',
		{ v: parseFloat(informe.egresosComparendos) || 0, estilo: { monto: true } }
	]);
	filas.push([
		{ v: 'Total egresos', estilo: { bold: true } },
		{ v: parseFloat(informe.totalEgresos) || 0, estilo: { bold: true, monto: true } }
	]);
	const balance = parseFloat(informe.balance) || 0;
	filas.push([
		{
			v: 'BALANCE',
			estilo: { bold: true, fontSize: 12, color: balance >= 0 ? '1B5E20' : 'B71C1C' }
		},
		{
			v: balance,
			estilo: { bold: true, fontSize: 12, monto: true, color: balance >= 0 ? '1B5E20' : 'B71C1C' }
		}
	]);
	filas.push([{ v: ' ' }]);

	// Gastos por categoría
	filas.push([{ v: 'GASTOS POR CATEGORÍA', estilo: { bold: true, fill: AZUL_CLARO } }]);
	filas.push([
		{ v: 'Categoría', estilo: { bold: true } },
		{ v: 'Monto', estilo: { bold: true } }
	]);
	for (const [categoria, total] of informe.gastosPorCategoria) {
		filas.push([categoria, { v: parseFloat(total) || 0, estilo: { monto: true } }]);
	}
	filas.push([{ v: ' ' }]);

	// Rentas del rango
	filas.push([
		{ v: `RENTAS DEL PERIODO (${informe.rentas.length})`, estilo: { bold: true, fill: AZUL_CLARO } }
	]);
	filas.push([
		{ v: 'No.', estilo: { bold: true } },
		{ v: 'Placa', estilo: { bold: true } },
		{ v: 'Cliente', estilo: { bold: true } },
		{ v: 'Fecha recogida', estilo: { bold: true } },
		{ v: 'Estado', estilo: { bold: true } },
		{ v: 'Total', estilo: { bold: true } }
	]);
	for (const r of informe.rentas) {
		filas.push([
			String(r.id),
			r.placa || '',
			r.nombreCliente,
			r.fechaRecogida,
			r.estado,
			{ v: parseFloat(r.total) || 0, estilo: { monto: true } }
		]);
	}
	filas.push([{ v: ' ' }]);

	// Utilidad por vehículo
	filas.push([
		{
			v: `UTILIDAD POR VEHÍCULO (${informe.utilidadPorVehiculo.length})`,
			estilo: { bold: true, fill: AZUL_CLARO }
		}
	]);
	filas.push([
		{ v: 'Placa', estilo: { bold: true } },
		{ v: 'Vehículo', estilo: { bold: true } },
		{ v: 'Ingresos', estilo: { bold: true } },
		{ v: 'Costos', estilo: { bold: true } },
		{ v: 'Utilidad', estilo: { bold: true } }
	]);
	for (const v of informe.utilidadPorVehiculo) {
		filas.push([
			v.placa,
			v.vehiculo || '',
			{ v: parseFloat(v.ingresos) || 0, estilo: { monto: true } },
			{ v: parseFloat(v.costos) || 0, estilo: { monto: true } },
			{ v: parseFloat(v.utilidad) || 0, estilo: { monto: true } }
		]);
	}

	return filas;
}

/**
 * Convierte las filas del informe en un Workbook .xlsx estilizado.
 * La celda (0,0) se fusiona como título y los encabezados de sección
 * ocupan el ancho de su tabla.
 *
 * @returns `ExcelJS.Workbook` síncrono (no requiere await para construirse).
 *   Llama `await wb.xlsx.writeBuffer()` para serializarlo a bytes.
 */
export function construirLibroInforme(informe: InformeMensual, periodo: string): ExcelJS.Workbook {
	const filas = filasInformeExcel(informe, periodo);
	const wb = new ExcelJS.Workbook();
	const ws = wb.addWorksheet('Informe');

	// ── Anchos de columna (ExcelJS es 1-indexed) ──
	ws.columns = ANCHOS_COLUMNAS.map((w) => ({ width: w }));

	// ── Volcado de filas + estilos por celda ──
	filas.forEach((fila, r) => {
		const row = ws.addRow(aplanar(fila));
		fila.forEach((celda, c) => {
			if (typeof celda !== 'object' || !celda.estilo) return;
			const cell = row.getCell(c + 1); // 1-indexed
			const { bold, fill, color, fontSize, monto } = celda.estilo;
			const font: Partial<ExcelJS.Font> = {};
			if (bold) font.bold = true;
			if (color) font.color = { argb: argb(color) };
			if (fontSize) font.size = fontSize;
			if (Object.keys(font).length > 0) cell.font = font as ExcelJS.Font;
			if (fill) {
				cell.fill = {
					type: 'pattern',
					pattern: 'solid',
					fgColor: { argb: argb(fill) }
				};
			}
			if (monto) {
				cell.numFmt = FORMATO_MONTO;
				cell.alignment = { horizontal: 'right' };
			}
		});
	});

	// ── Fusiones: título (fila 1) y periodo (fila 2) a lo ancho del libro ──
	const anchoLibro = ANCHOS_COLUMNAS.length; // número de columnas
	ws.mergeCells(1, 1, 1, anchoLibro);
	ws.mergeCells(2, 1, 2, anchoLibro);

	// ── Fusiones de encabezados de sección (ancho de su tabla) ──
	filas.forEach((fila, r) => {
		const primera = fila[0];
		if (typeof primera !== 'object' || typeof primera.v !== 'string') return;
		const clave = String(primera.v);
		for (const [prefijo, ultimaCol] of SECCIONES) {
			if (clave.startsWith(prefijo)) {
				// ExcelJS: (top, left, bottom, right), 1-indexed
				ws.mergeCells(r + 1, 1, r + 1, ultimaCol + 1);
				break;
			}
		}
	});

	return wb;
}
