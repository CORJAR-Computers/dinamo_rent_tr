#!/usr/bin/env node
// scripts/test-verificar-paginacion.mjs — Test de verificar-paginacion.mjs
//
// Ejecuta el verificador de paginación contra los fixtures de demostración y
// exige que cada documento cumpla su expectativa real (las mismas que se
// usan a mano y en el README):
//
//   1. una-pagina.html   → exactamente 1 página Carta (orden de renta; no
//                          lleva pie «Página X de Y»: ese pie es del contrato).
//   2. tres-paginas.html → entre 3 y 4 páginas Carta CON pie «Página X de Y».
//   3. a4.html           → hoja A4 (informe mensual; sin expectativa de
//                          páginas, solo el tamaño).
//   4. reserva.html      → exactamente 1 página Carta (orden de reserva;
//                          tampoco lleva pie: es del contrato).
//
// El verificador necesita un navegador Chromium (Chrome/Edge headless): en
// CI (windows-latest) Edge viene instalado; localmente se usa el del sistema
// o DINAMO_BROWSER_BIN. Si no hay navegador, el test falla con mensaje claro
// (no es un falso verde).
//
// Uso: node scripts/test-verificar-paginacion.mjs   (npm run test:paginacion)

import { spawn } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const VERIFICADOR = join(__dirname, 'verificar-paginacion.mjs');

const fallos = [];
const ok = (m) => console.log(`  ✓ ${m}`);
const falla = (m) => {
	console.log(`  ✗ ${m}`);
	fallos.push(m);
};

/** Ejecuta el verificador con un fixture y devuelve { codigo, stdout }. */
function ejecutar(archivo, exp, opts) {
	return new Promise((resolve) => {
		const ruta = join(__dirname, 'fixtures', archivo);
		if (!existsSync(ruta)) {
			resolve({ codigo: -1, stdout: `fixture no existe: ${ruta}` });
			return;
		}
		const args = [VERIFICADOR, `${ruta}${exp}`, ...opts];
		const child = spawn(process.execPath, args, { stdio: ['ignore', 'pipe', 'inherit'] });
		let stdout = '';
		child.stdout.on('data', (d) => (stdout += d));
		child.on('error', (e) => resolve({ codigo: -1, stdout: `no se pudo lanzar node: ${e.message}` }));
		child.on('close', (codigo) => resolve({ codigo, stdout }));
	});
}

/** Un caso: el verificador debe salir 0 (0 = todo OK; 1 = técnico, 2 = falla). */
async function caso(nombre, archivo, exp, opts, detalle) {
	console.log(`\n${nombre}`);
	const r = await ejecutar(archivo, exp, opts);
	console.log(`  exit=${r.codigo}`);
	if (r.codigo === 0) ok(detalle);
	else if (r.codigo === 1) falla(`error técnico del verificador (¿hay navegador Chromium? ${detalle})`);
	else falla(`verificación fallida — ${detalle}`);
}

// ───────────────────────────────── Main ─────────────────────────────────

console.log('Verificando verificar-paginacion.mjs contra los fixtures…');

await caso(
	'1. Orden de renta — 1 página Carta (sin pie)',
	'una-pagina.html',
	'=1',
	['--tamano', 'carta'],
	'exactamente 1 página en papel Carta'
);

await caso(
	'2. Contrato — 3 a 4 páginas Carta con pie «Página X de Y»',
	'tres-paginas.html',
	'=3:4',
	['--tamano', 'carta', '--pie'],
	'3-4 páginas Carta y pie de página correcto en todas'
);

await caso(
	'3. Informe mensual — hoja A4',
	'a4.html',
	'',
	['--tamano', 'a4'],
	'MediaBox A4 (595×842 pt)'
);

await caso(
	'4. Orden de reserva — 1 página Carta (sin pie)',
	'reserva.html',
	'=1',
	['--tamano', 'carta'],
	'exactamente 1 página en papel Carta'
);

console.log('');
const TOTAL = 4;
if (fallos.length === 0) {
	console.log(`Resultado: ${TOTAL}/${TOTAL} casos OK`);
	process.exit(0);
} else {
	console.log(`Resultado: ${TOTAL - fallos.length}/${TOTAL} casos OK · ${fallos.length} fallo(s):`);
	for (const f of fallos) console.log(`  - ${f}`);
	process.exit(1);
}
