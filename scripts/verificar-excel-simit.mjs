// verificar-excel-simit.mjs — Verificación de punta a punta del export Excel
// de comparendos SIMIT.
//
// Lee el resultado de una sincronización (data/simit_watch/sync_result.json,
// dump de `cargo run --features dev --bin sync_dev`) y construye el .xlsx con
// EXACTAMENTE el mismo mapeo que el botón «Exportar Excel» de
// src/routes/comparendos/+page.svelte (descargarExcelSimit), usando el mismo
// exceljs del frontend. Luego reabre el libro y valida el contenido real.
//
// Uso: node scripts/verificar-excel-simit.mjs
// Salida: data/informes_simit/comparendos_simit_<fecha>.xlsx (verificado)

import ExcelJS from 'exceljs';
import fs from 'node:fs';
import path from 'node:path';

const RAÍZ = new URL('..', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const RESULTADO = path.join(RAÍZ, 'data', 'simit_watch', 'sync_result.json');
const SALIDA_DIR = path.join(RAÍZ, 'data', 'informes_simit');

if (!fs.existsSync(RESULTADO)) {
	console.error(`✗ No existe ${RESULTADO}. Corre primero: cargo run --features dev --bin sync_dev`);
	process.exit(1);
}

const resultado = JSON.parse(fs.readFileSync(RESULTADO, 'utf8'));
const fallos = [];
const ok = (m) => console.log(`  ✓ ${m}`);
const falla = (m) => fallos.push(m);

console.log(`\nExport Excel SIMIT — resultado de ${resultado.sincronizadoEn}`);
console.log(`  registros en el resultado: ${resultado.registros.length} · total pendiente: $${resultado.totalPendiente}`);

// ── 1. Construir el libro (mismo mapeo que descargarExcelSimit) ──────────────
const wb = new ExcelJS.Workbook();
const ws = wb.addWorksheet('Comparendos SIMIT');
ws.columns = Array.from({ length: 10 }, () => ({ width: 16 }));
ws.mergeCells(1, 1, 1, 10);
ws.getCell('A1').value = 'DINAMO RENT — REPORTE SIMIT';
ws.getCell('A1').font = { bold: true, size: 14 };
ws.mergeCells(2, 1, 2, 10);
ws.getCell('A2').value = `Sincronización: ${new Date(resultado.sincronizadoEn).toLocaleString('es-CO')}`;
ws.addRow([]);
const header = ['Placa', 'N° Comparendo', 'Fecha', 'Hora', 'Tipo', 'Infracción', 'Organismo', 'Valor', 'Estado', 'Nuevo'];
const headerRow = ws.addRow(header);
headerRow.eachCell((cell) => {
	cell.font = { bold: true };
	cell.fill = { type: 'pattern', pattern: 'solid', fgColor: { argb: 'FFE8EAF6' } };
});
for (const r of resultado.registros) {
	ws.addRow([
		r.placa,
		r.numero ?? '',
		r.fechaInfraccion,
		r.horaInfraccion,
		r.esComparendo ? 'Comparendo' : 'Multa',
		r.descripcion,
		r.organismo,
		parseFloat(r.monto) || 0,
		r.estado,
		r.nuevo ? 'Sí' : 'No'
	]);
}
ws.addRow([]);
ws.addRow(['Total pendiente', `$${resultado.totalPendiente}`]);

// ── 2. Escribir el archivo real ──────────────────────────────────────────────
fs.mkdirSync(SALIDA_DIR, { recursive: true });
const fecha = String(resultado.sincronizadoEn).slice(0, 10);
const salida = path.join(SALIDA_DIR, `comparendos_simit_${fecha}.xlsx`);
const buffer = await wb.xlsx.writeBuffer();
fs.writeFileSync(salida, buffer);
console.log(`\n  Libro escrito → ${salida} (${buffer.length} bytes)\n`);

// ── 3. Reabrir y validar el contenido ────────────────────────────────────────
const wb2 = new ExcelJS.Workbook();
await wb2.xlsx.load(buffer);
const ws2 = wb2.getWorksheet('Comparendos SIMIT');
if (!ws2) {
	falla('No se encontró la hoja "Comparendos SIMIT"');
} else {
	// Título
	const a1 = ws2.getCell('A1').value;
	if (String(a1) !== 'DINAMO RENT — REPORTE SIMIT') {
		falla(`A1 no es el título esperado: "${a1}"`);
	} else {
		ok('A1 título: DINAMO RENT — REPORTE SIMIT');
	}

	// Encabezados (fila 4)
	const filaHeader = ws2.getRow(4);
	const headers = [];
	for (let c = 1; c <= 10; c++) headers.push(String(filaHeader.getCell(c).value ?? ''));
	if (JSON.stringify(headers) !== JSON.stringify(header)) {
		falla(`Encabezados no coinciden:\n  esperado: ${JSON.stringify(header)}\n  obtenido: ${JSON.stringify(headers)}`);
	} else {
		ok('Encabezados (10 columnas) correctos');
	}

	// Datos: filas 5..(4+n)
	const n = resultado.registros.length;
	const filasDatos = [];
	for (let r = 5; r < 5 + n; r++) {
		const row = ws2.getRow(r);
		filasDatos.push({
			placa: String(row.getCell(1).value ?? ''),
			numero: String(row.getCell(2).value ?? ''),
			fecha: String(row.getCell(3).value ?? ''),
			valor: row.getCell(8).value,
			estado: String(row.getCell(9).value ?? ''),
			nuevo: String(row.getCell(10).value ?? '')
		});
	}
	if (filasDatos.length !== n) {
		falla(`Filas de datos: ${filasDatos.length} (esperado ${n})`);
	} else {
		ok(`Filas de datos: ${n} registros exportados`);
	}

	// Comparación registro a registro contra el JSON
	let mismatches = 0;
	resultado.registros.forEach((r, i) => {
		const d = filasDatos[i];
		if (d.placa !== r.placa || d.fecha !== r.fechaInfraccion || d.estado !== r.estado) {
			mismatches++;
			if (mismatches <= 3) {
				falla(`Fila ${i}: ${d.placa} ${d.fecha} ${d.estado} ≠ ${r.placa} ${r.fechaInfraccion} ${r.estado}`);
			}
		}
	});
	if (mismatches === 0) ok('Los N registros coinciden 1:1 con el resultado (placa/fecha/estado)');

	// Valores numéricos de monto
	let montosOk = 0;
	resultado.registros.forEach((r, i) => {
		const esperado = parseFloat(r.monto) || 0;
		if (filasDatos[i].valor === esperado) montosOk++;
	});
	if (montosOk === n) ok(`Montos como número (${n}/${n} celdas)`);
	else falla(`Montos: ${montosOk}/${n} celdas coinciden`);

	// Total pendiente (fila 5+n+1 = después del vacío)
	const filaTotal = ws2.getRow(5 + n + 1);
	const totalCelda = String(filaTotal.getCell(1).value ?? '');
	const totalValor = String(filaTotal.getCell(2).value ?? '');
	if (totalCelda !== 'Total pendiente' || totalValor !== `$${resultado.totalPendiente}`) {
		falla(`Total pendiente: "${totalCelda}" / "${totalValor}" (esperado "$${resultado.totalPendiente}")`);
	} else {
		ok(`Total pendiente: ${totalValor}`);
	}
}

// ── 4. Resumen ──────────────────────────────────────────────────────────────
console.log('');
if (fallos.length === 0) {
	console.log(`✅ EXPORT EXCEL VALIDADO — ${salida}`);
	process.exit(0);
} else {
	console.error(`✗ Export Excel con ${fallos.length} problema(s):`);
	for (const f of fallos) console.error(`  - ${f}`);
	process.exit(1);
}
