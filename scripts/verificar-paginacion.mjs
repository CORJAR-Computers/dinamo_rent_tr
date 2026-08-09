#!/usr/bin/env node
/**
 * verificar-paginacion.mjs — Verificador reutilizable de paginación de documentos.
 *
 * Flujo: HTML estático → Chrome/Edge headless → PDF → contador de páginas
 * (con verificación opcional de tamaño de hoja y del pie «Página X de Y»).
 *
 * También acepta **PDFs ya generados** (p. ej. los que produce el smoke test
 * de la app, `smoke-test-app.mjs`): si el archivo termina en `.pdf` se analiza
 * directamente, sin pasar por el navegador.
 *
 * Uso:
 *   node scripts/verificar-paginacion.mjs <archivo.html|archivo.pdf> [más...] [opciones]
 *
 * Expectativas por archivo (sufijo `=`):
 *   archivo.html        → solo informa la paginación
 *   archivo.html=3      → exige exactamente 3 páginas
 *   archivo.html=3:4    → exige entre 3 y 4 páginas (inclusive)
 *
 * Opciones:
 *   --motor auto|chrome|edge   Motor a usar (auto: Edge → Chrome → sistema).
 *   --bin <ruta>               Binario explícito del navegador (o env DINAMO_BROWSER_BIN).
 *   --tamano carta|letter|a4|AxB   Verifica la hoja (MediaBox del PDF) contra ese tamaño.
 *   --pie                      Verifica el pie «Página X de Y» (@page margin box).
 *   --headers                  Simula el diálogo con «Encabezados y pies de página» ACTIVADO.
 *   --salida <dir>             Directorio donde guardar los PDFs (se conservan).
 *   --conservar                Conserva los PDFs temporales generados.
 *   --tiempo <ms>              Tiempo máximo de impresión por archivo (def. 90000).
 *   --uid <dir>                Perfil (user-data-dir) personalizado del navegador.
 *   --ayuda | -h               Muestra esta ayuda.
 *
 * Códigos de salida:
 *   0  todo OK
 *   1  error técnico (navegador no encontrado, fallo de impresión, timeout, archivo inexistente)
 *   2  verificación fallida (expectativa de páginas, tamaño o pie incorrectos)
 *
 * Ejemplos:
 *   node scripts/verificar-paginacion.mjs scripts/fixtures/una-pagina.html
 *   node scripts/verificar-paginacion.mjs scripts/fixtures/una-pagina.html=1
 *   node scripts/verificar-paginacion.mjs scripts/fixtures/tres-paginas.html=3:4 --tamano carta --pie
 *   node scripts/verificar-paginacion.mjs contrato.html=3:4 orden.html=1 --tamano carta
 *   node scripts/verificar-paginacion.mjs contrato.html=3:4 --motor edge --salida ./pdfs
 *   node scripts/verificar-paginacion.mjs .tmp-print/contrato-real.pdf=3:4 --tamano carta --pie
 */

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, delimiter as pathDelim, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import zlib from 'node:zlib';

const AYUDA = `Verificador reutilizable de paginación de documentos.

Uso:
  node scripts/verificar-paginacion.mjs <archivo.html|archivo.pdf> [más...] [opciones]

Los archivos .pdf se analizan directamente; los .html se imprimen con
Chrome/Edge headless para generar el PDF y luego se analizan.

Expectativas por archivo (sufijo \`=\`):
  archivo.html        solo informa la paginación
  archivo.html=3      exige exactamente 3 páginas
  archivo.html=3:4    exige entre 3 y 4 páginas (inclusive)

Opciones:
  --motor auto|chrome|edge   Motor a usar (auto: Edge → Chrome → sistema)
  --bin <ruta>               Binario explícito (o variable de entorno DINAMO_BROWSER_BIN)
  --tamano carta|letter|a4|AxB   Verifica la hoja (MediaBox) contra ese tamaño
  --pie                      Verifica el pie «Página X de Y» (@page margin box)
  --headers                  Simula el diálogo con «Encabezados y pies» ACTIVADO
  --salida <dir>             Directorio donde guardar los PDFs (se conservan)
  --conservar                Conserva los PDFs temporales generados
  --tiempo <ms>              Tiempo máximo por archivo (def. 90000)
  --uid <dir>                Perfil (user-data-dir) personalizado
  --ayuda | -h               Muestra esta ayuda

Códigos de salida:
  0  todo OK
  1  error técnico (navegador no encontrado, fallo de impresión, timeout...)
  2  verificación fallida (páginas, tamaño o pie incorrectos)

Ejemplos:
  node scripts/verificar-paginacion.mjs scripts/fixtures/una-pagina.html=1
  node scripts/verificar-paginacion.mjs scripts/fixtures/tres-paginas.html=3:4 --tamano carta --pie
  node scripts/verificar-paginacion.mjs contrato.html=3:4 orden.html=1 --tamano carta
  node scripts/verificar-paginacion.mjs contrato.html=3:4 --motor edge --salida ./pdfs
  node scripts/verificar-paginacion.mjs .tmp-print/contrato-real.pdf=3:4 --tamano carta --pie`;

/** Tamaños de hoja conocidos en puntos (1 pt = 1/72 pulgada). */
const TAMANOS = {
	carta: [612, 792],
	letter: [612, 792],
	a4: [595, 842]
};

/** Binarios Chromium por plataforma, en orden de preferencia. */
const CANDIDATOS = {
	chrome: [
		'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
		'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
		join(process.env.LOCALAPPDATA || '', 'Google', 'Chrome', 'Application', 'chrome.exe'),
		'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
		'/usr/bin/google-chrome',
		'/usr/bin/google-chrome-stable',
		'/usr/bin/chromium',
		'/usr/bin/chromium-browser'
	],
	edge: [
		'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
		'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
		'/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
		'/usr/bin/microsoft-edge',
		'/usr/bin/microsoft-edge-stable'
	]
};

/**
 * Expresión del pie: «Página N de M».
 * Edge/Chrome dibujan el texto del margen @page glifo por glifo (cada letra en
 * un run propio, unidos por espacios) y la fuente subconjunto corrompe las
 * letras acentuadas (p. ej. «á»); por eso se permite espacios entre letras y
 * un carácter opcional en la posición del acento.
 */
const RE_PIE = /P\s*[^\s]?\s*g\s*i\s*n\s*a\s*(\d+)\s*d\s*e\s*(\d+)/g;

// ─────────────────────────── Línea de comandos ───────────────────────────

function parseArgs(argv) {
	const opts = {
		motor: 'auto',
		bin: null,
		tamano: null,
		pie: false,
		headers: false,
		salida: null,
		conservar: false,
		tiempo: 90000,
		uid: null,
		ayuda: false
	};
	// `leerValor` incrementa el índice de argv; por eso el bucle usa un `let i`
	// de ámbito de función (un `for (let i ...)` crearía bindings inmutables
	// por iteración y el `++i` interno lanzaría «Assignment to constant»).
	const leerValor = (a) => {
		const eq = a.indexOf('=');
		return eq !== -1 ? a.slice(eq + 1) : argv[++i];
	};
	const archivos = [];
	let i = 0;
	while (i < argv.length) {
		const a = argv[i];
		if (a === '--ayuda' || a === '-h' || a === '--help') {
			opts.ayuda = true;
		} else if (a === '--pie') {
			opts.pie = true;
		} else if (a === '--headers' || a === '--con-encabezados') {
			opts.headers = true;
		} else if (a === '--conservar') {
			opts.conservar = true;
		} else if (a.startsWith('--motor')) {
			opts.motor = leerValor(a);
		} else if (a.startsWith('--bin')) {
			opts.bin = leerValor(a);
		} else if (a.startsWith('--tamano')) {
			opts.tamano = leerValor(a);
		} else if (a.startsWith('--salida')) {
			opts.salida = leerValor(a);
		} else if (a.startsWith('--tiempo')) {
			opts.tiempo = Number(leerValor(a)) || 90000;
		} else if (a.startsWith('--uid')) {
			opts.uid = leerValor(a);
		} else if (a.startsWith('-')) {
			console.error(`Opción desconocida: ${a}\n\n${AYUDA}`);
			process.exit(1);
		} else {
			archivos.push(a);
		}
		i++;
	}
	return { opts, archivos };
}

/** Separa `ruta.html=min:max` en { ruta, exp: [min, max] | null }. */
function parseArchivo(item) {
	const eq = item.indexOf('=');
	if (eq === -1) return { ruta: item, exp: null };
	const ruta = item.slice(0, eq);
	const parte = item.slice(eq + 1);
	if (!parte) return { ruta, exp: null };
	const [a, b] = parte.split(':').map(Number);
	if (Number.isNaN(a)) return { ruta, exp: null };
	return { ruta, exp: b !== undefined && !Number.isNaN(b) ? [a, b] : [a, a] };
}

function resolverTamano(t) {
	if (!t) return null;
	const clave = t.toLowerCase();
	if (TAMANOS[clave]) return TAMANOS[clave];
	const m = clave.match(/^(\d+)x(\d+)$/);
	return m ? [Number(m[1]), Number(m[2])] : null;
}

// ────────────────────────────── Navegador ────────────────────────────────

function buscarEnPath(nombre) {
	const sufijo = process.platform === 'win32' ? '.exe' : '';
	const ruta = (process.env.PATH || '')
		.split(pathDelim)
		.map((p) => join(p, nombre + sufijo));
	return ruta.find((p) => existsSync(p)) || null;
}

function buscarBinario(motor) {
	const env = process.env.DINAMO_BROWSER_BIN;
	if (env && existsSync(env)) return env;
	const orden = motor === 'auto' ? ['edge', 'chrome'] : [motor];
	for (const m of orden) {
		for (const p of CANDIDATOS[m]) {
			if (existsSync(p)) return p;
		}
		const bin = m === 'edge' ? 'msedge' : 'google-chrome';
		const enPath = buscarEnPath(bin);
		if (enPath) return enPath;
	}
	return null;
}

/** Imprime una URL con Chrome/Edge headless y espera el PDF. */
function imprimir(bin, args, timeoutMs) {
	return new Promise((resolver) => {
		let err = '';
		let terminado = false;
		const fin = (ok, msg) => {
			if (!terminado) {
				terminado = true;
				resolver({ ok, msg });
			}
		};
		const child = spawn(bin, args, { stdio: ['ignore', 'ignore', 'pipe'] });
		child.stderr.on('data', (d) => (err += d));
		child.on('error', (e) => fin(false, `no se pudo iniciar el navegador: ${e.message}`));
		child.on('close', (code) =>
			fin(code === 0, code === 0 ? '' : `el navegador salió con código ${code}: ${err.slice(-400).trim()}`)
		);
		const t = setTimeout(() => {
			child.kill('SIGKILL');
			fin(false, `tiempo agotado (${timeoutMs} ms)`);
		}, timeoutMs);
		t.unref?.();
		child.once('close', () => clearTimeout(t));
	});
}

// ────────────────────────── Análisis del PDF ─────────────────────────────

/** Cuenta páginas: /Type /Page del árbol (con /Count de /Pages como respaldo). */
function contarPaginas(pdf) {
	const raw = pdf.toString('latin1');
	const tipo = (raw.match(/\/Type\s*\/Page(?!s)/g) || []).length;
	const mCount = raw.match(/\/Count\s+(\d+)/);
	const countObj = mCount ? Number(mCount[1]) : null;
	if (tipo === 0 && countObj !== null) return { paginas: countObj, aviso: null };
	if (countObj !== null && countObj !== tipo) {
		return {
			paginas: countObj,
			aviso: `aviso: /Type /Page=${tipo} y /Count=${countObj} no coinciden; se usa /Count`
		};
	}
	return { paginas: tipo, aviso: null };
}

/** Detecta el tamaño de hoja a partir del MediaBox (pt, redondeado). */
function detectarTamano(pdf) {
	const raw = pdf.toString('latin1');
	const m = raw.match(/\/MediaBox\s*\[\s*0\s+0\s+(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)\s*\]/);
	return m ? { w: Math.round(Number(m[1])), h: Math.round(Number(m[2])) } : null;
}

function etiquetaTamano(t) {
	if (t.w === 612 && t.h === 792) return '612×792 pt (Carta)';
	if (t.w === 595 && t.h === 842) return '595×842 pt (A4)';
	return `${t.w}×${t.h} pt`;
}

/**
 * Descomprime los flujos del PDF (best-effort) para extraer texto.
 * Se omiten los flujos grandes (>512 KB): son fuentes/imágenes embebidas, no
 * texto; inflarlos (p. ej. en PDFs reales con logo) encarece la búsqueda de
 * pie de página (256 pases de decodificación).
 */
function inflar(pdf) {
	const raw = pdf.toString('latin1');
	let out = '';
	const re = /stream\r?\n([\s\S]*?)endstream/g;
	let m;
	while ((m = re.exec(raw))) {
		if (m[1].length > 512 * 1024) continue;
		try {
			out += zlib.inflateSync(Buffer.from(m[1], 'latin1')).toString('latin1');
		} catch {
			/* flujo no comprimido o corrupto: se omite */
		}
	}
	return out;
}

/**
 * Extrae texto de los operadores Tj/TJ. Los códigos hex de 2 bytes usan una
 * fuente CID subconjunto con desplazamiento constante por fuente; `shift` lo
 * corrige (0..255). Las cadenas literales `(...)` no necesitan corrección.
 */
function decodificarTexto(s, shift) {
	let dec = '';
	const re = /\[([\s\S]*?)\]\s*TJ|\((?:\\\.|[^\\()])*\)\s*Tj|<[0-9a-fA-F\s]*>\s*Tj/g;
	let t;
	while ((t = re.exec(s))) {
		const partes = t[0].match(/\((?:\\\.|[^\\()])*\)|<[0-9a-fA-F\s]*>/g) || [];
		for (const p of partes) {
			if (p.startsWith('(')) {
				dec += p.slice(1, -1).replace(/\\([()\\])/g, '$1');
			} else {
				const hex = p.slice(1, -1).replace(/\s+/g, '');
				const b = hex.match(/.{1,2}/g) || [];
				for (let i = 0; i + 1 < b.length; i += 2) {
					dec += String.fromCharCode(((parseInt(b[i + 1], 16) || 0) + shift) & 0xff);
				}
			}
		}
		dec += ' ';
	}
	return dec;
}

/**
 * Verifica el pie «Página X de Y» en las páginas del PDF.
 * Prueba los 256 desplazamientos de decodificación y se queda con el que
 * más coincidencias produce (best-effort: depende de la fuente del margen box).
 */
function verificarPie(pdf, paginas) {
	const inflado = inflar(pdf);
	let mejor = [];
	for (let s = 0; s < 256; s++) {
		const texto = decodificarTexto(inflado, s);
		const m = [...texto.matchAll(RE_PIE)];
		if (m.length > mejor.length) mejor = m;
	}
	if (mejor.length === 0) {
		return {
			ok: false,
			detalle: 'pie «Página X de Y» no detectado (método de extracción best-effort)'
		};
	}
	const nums = mejor.map((x) => Number(x[1]));
	const tots = mejor.map((x) => Number(x[2]));
	const secuencial = nums.every((n, i) => n === i + 1);
	const totalOk = tots.every((t) => t === paginas);
	const completo = mejor.length === paginas;
	if (!secuencial || !totalOk || !completo) {
		const encontrado = mejor.map((x) => x[0]).join(' · ');
		return { ok: false, detalle: `pie inconsistente: ${encontrado} (esperado 1..${paginas} de ${paginas})` };
	}
	return { ok: true, detalle: `${mejor.length}/${paginas} páginas con «Página X de Y» correcto` };
}

/**
 * Analiza un buffer de PDF: páginas, tamaño de hoja y pie opcional.
 * Devuelve true si alguna expectativa falló.
 */
function analizarPDF(buf, nombre, exp, esperado, opts) {
	const { paginas, aviso } = contarPaginas(buf);
	const tam = detectarTamano(buf);
	let falloArchivo = false;

	const lineaPaginas = `${paginas} página${paginas === 1 ? '' : 's'}`;
	if (exp) {
		const [min, max] = exp;
		const ok = paginas >= min && paginas <= max;
		console.log(`  ${ok ? '[OK]' : '[FALLO]'} páginas: ${lineaPaginas} (esperado ${min === max ? min : `${min}:${max}`})`);
		if (!ok) falloArchivo = true;
	} else {
		console.log(`  [OK] páginas: ${lineaPaginas} (sin expectativa)`);
	}
	if (aviso) console.log(`  ${aviso}`);

	if (tam) {
		const ok = !esperado || (tam.w === esperado[0] && tam.h === esperado[1]);
		const etiqueta = ok ? '[OK]' : '[FALLO]';
		console.log(`  ${etiqueta} tamaño: ${etiquetaTamano(tam)}${opts.tamano ? ` (esperado ${opts.tamano})` : ''}`);
		if (!ok) falloArchivo = true;
	} else if (esperado) {
		console.log(`  [FALLO] tamaño: no se pudo detectar el MediaBox (esperado ${opts.tamano})`);
		falloArchivo = true;
	} else {
		console.log('  [OK] tamaño: no detectado (sin expectativa)');
	}

	if (opts.pie) {
		const r = verificarPie(buf, paginas);
		console.log(`  ${r.ok ? '[OK]' : '[FALLO]'} pie: ${r.detalle}`);
		if (!r.ok) falloArchivo = true;
	}

	return falloArchivo;
}

// ────────────────────────────────── Main ─────────────────────────────────

async function main() {
	const { opts, archivos } = parseArgs(process.argv.slice(2));
	if (opts.ayuda) {
		console.log(AYUDA);
		process.exit(0);
	}
	if (archivos.length === 0) {
		console.error('Indica al menos un archivo HTML o PDF.\n\n' + AYUDA);
		process.exit(1);
	}
	if (!['auto', 'chrome', 'edge'].includes(opts.motor)) {
		console.error(`--motor inválido: "${opts.motor}" (usa auto, chrome o edge).`);
		process.exit(1);
	}
	const esperado = resolverTamano(opts.tamano);
	if (opts.tamano && !esperado) {
		console.error(`--tamano inválido: "${opts.tamano}" (usa carta, letter, a4 o Ancho x Alto).`);
		process.exit(1);
	}

	// Si todos los archivos son PDFs ya generados, no hace falta el navegador.
	const soloPDFs = archivos.every((a) => /\.pdf$/i.test(parseArchivo(a).ruta));
	const binario = soloPDFs ? null : (opts.bin ?? buscarBinario(opts.motor));
	if (!soloPDFs && !binario) {
		console.error(
			'No se encontró un navegador Chromium (Chrome o Edge).\n' +
				'   Instala uno o indica la ruta con --bin <ruta> o la variable DINAMO_BROWSER_BIN.'
		);
		process.exit(1);
	}

	const work = opts.salida ? resolve(opts.salida) : mkdtempSync(join(tmpdir(), 'dinamo-verif-'));
	mkdirSync(work, { recursive: true });
	const uidDir = opts.uid ? resolve(opts.uid) : mkdtempSync(join(tmpdir(), 'dinamo-perfil-'));

	console.log(soloPDFs ? 'Modo PDF directo (sin navegador)' : `Navegador: ${binario}`);
	console.log(`PDFs: ${opts.salida ? work : '(temporales)'}\n`);

	let erroresTecnicos = 0;
	let fallosVerificacion = 0;
	const nArchivos = archivos.length;

	try {
		for (const item of archivos) {
			const { ruta, exp } = parseArchivo(item);
			const nombre = basename(ruta);
			console.log(`== ${nombre} ==`);
			if (!existsSync(ruta)) {
				console.error('  [ERROR] el archivo no existe\n');
				erroresTecnicos++;
				continue;
			}

			// PDF ya generado (p. ej. por smoke-test-app.mjs): análisis directo.
			if (/\.pdf$/i.test(ruta)) {
				const falloPDF = analizarPDF(readFileSync(ruta), nombre, exp, esperado, opts);
				console.log(falloPDF ? '  → verificación fallida\n' : '  → verificación superada\n');
				if (falloPDF) fallosVerificacion++;
				continue;
			}

			const pdf = join(work, nombre.replace(/\.[^.]+$/, '') + '.pdf');
			const args = [
				'--headless=new',
				'--no-sandbox',
				'--disable-gpu',
				`--user-data-dir=${uidDir}`,
				...(opts.headers ? [] : ['--no-pdf-header-footer']),
				`--print-to-pdf=${pdf}`,
				pathToFileURL(resolve(ruta)).href
			];
			const res = await imprimir(binario, args, opts.tiempo);
			if (!res.ok || !existsSync(pdf) || statSync(pdf).size === 0) {
				console.error(`  [ERROR] no se generó el PDF: ${res.msg || 'archivo vacío'}\n`);
				erroresTecnicos++;
				continue;
			}

			const falloArchivo = analizarPDF(readFileSync(pdf), nombre, exp, esperado, opts);
			console.log(falloArchivo ? '  → verificación fallida\n' : '  → verificación superada\n');
			if (falloArchivo) fallosVerificacion++;
		}
	} finally {
		rmSync(uidDir, { recursive: true, force: true });
		if (!opts.salida && !opts.conservar) rmSync(work, { recursive: true, force: true });
	}

	const okTotal = nArchivos - erroresTecnicos - fallosVerificacion;
	console.log(
		`Resumen: ${okTotal}/${nArchivos} archivos OK` +
			(fallosVerificacion ? ` · ${fallosVerificacion} verificación(es) fallida(s)` : '') +
			(erroresTecnicos ? ` · ${erroresTecnicos} error(es) técnico(s)` : '') +
			(opts.salida ? ` · PDFs en ${work}` : opts.conservar ? ` · PDFs temporales en ${work}` : '')
	);
	process.exit(erroresTecnicos > 0 ? 1 : fallosVerificacion > 0 ? 2 : 0);
}

main().catch((e) => {
	console.error('Error inesperado:', e);
	process.exit(1);
});
