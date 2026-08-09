// informeExcel.ts — Exportación .xlsx real (SheetJS) del informe financiero.
// Funciones puras: `filasInformeExcel` produce las filas (testeable) y
// `construirLibroInforme` las convierte en un Workbook estilizado (título,
// encabezados con color, formato de moneda, anchos de columna y totales).
import * as XLSX from 'xlsx';
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
const ANCHOS_COLUMNAS: XLSX.ColInfo[] = [
	{ wch: 42 }, // Concepto / Cliente / Placa (utilidad)
	{ wch: 18 }, // Monto / Placa / Vehículo
	{ wch: 16 }, // Monto categorías / Total renta / Ingresos
	{ wch: 14 }, // Fecha / Costos
	{ wch: 14 }, // Estado / Utilidad
	{ wch: 14 } // Total
];

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
 */
export function construirLibroInforme(informe: InformeMensual, periodo: string): XLSX.WorkBook {
	const filas = filasInformeExcel(informe, periodo);
	const ws = XLSX.utils.aoa_to_sheet(filas.map(aplanar));
	ws['!cols'] = ANCHOS_COLUMNAS;

	// ── Fusiones: título (fila 0) y periodo (fila 1) a lo ancho del libro ──
	const anchoLibro = ANCHOS_COLUMNAS.length - 1;
	ws['!merges'] = [
		{ s: { r: 0, c: 0 }, e: { r: 0, c: anchoLibro } },
		{ s: { r: 1, c: 0 }, e: { r: 1, c: anchoLibro } }
	];

	// ── Estilos por celda ──
	filas.forEach((fila, r) => {
		fila.forEach((celda, c) => {
			if (typeof celda !== 'object' || !celda.estilo) return;
			const addr = XLSX.utils.encode_cell({ r, c });
			const cell = ws[addr];
			if (!cell) return;
			const { bold, fill, color, fontSize, monto } = celda.estilo;
			cell.s = {
				font: {
					...(bold ? { bold: true } : {}),
					...(color ? { color: { rgb: color } } : {}),
					...(fontSize ? { sz: fontSize } : {})
				},
				...(fill ? { fill: { patternType: 'solid', fgColor: { rgb: fill } } } : {}),
				...(monto
					? { numFmt: FORMATO_MONTO, alignment: { horizontal: 'right' } }
					: {})
			};
		});
	});

	// ── Fusiones de encabezados de sección (ancho de su tabla) ──
	filas.forEach((fila, r) => {
		const primera = fila[0];
		if (typeof primera !== 'object' || typeof primera.v !== 'string') return;
		const clave = String(primera.v);
		for (const [prefijo, ultimaCol] of SECCIONES) {
			if (clave.startsWith(prefijo)) {
				ws['!merges']!.push({ s: { r, c: 0 }, e: { r, c: ultimaCol } });
				break;
			}
		}
	});

	const wb = XLSX.utils.book_new();
	XLSX.utils.book_append_sheet(wb, ws, 'Informe');
	return wb;
}
