#!/usr/bin/env node
// smoke-dev.mjs — Smoke E2E en dev con BD AISLADA y VACÍA de rentas.
//
// A diferencia de `smoke:app` (que corre contra la BD que haya), este
// orquestador garantiza el escenario completo del flujo de cobro:
//
//   1. Compila app + seed_ci (`cargo build --features dev --bins`; el
//      feature `dev` es vacío y solo excluye esos binarios del bundle de
//      release — sin él `cargo build` no produce seed_ci.exe, corrida 287).
//   2. Crea un data_dir temporal (`scripts/.tmp-smoke-data`, ignorado por
//      git) y lo siembra con `seed_ci <dir>`: config.ini + BD con
//      admin/autos/clientes, pero SIN rentas → /rentas arranca vacía y el
//      smoke entra al branch de "renta de prueba" (el único que conoce la
//      base monetaria y puede asertar los totales de la extensión).
//   3. Levanta vite (dev server) y lanza el EXE ya compilado con
//      DINAMO_DATA_DIR apuntando al dir temporal y CDP en 9222
//      (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS).
//
//      ⚠️ TOKENS Y RUTAS DE WINDOWS EN CI (descubierto a fuerza de
//      corridas):
//      a) En un proceso ELEVADO, WebView2 IGNORA --remote-debugging-port
//         (probado con UAC: el mismo exe sin elevar abre 9222 en 1 s y
//         elevado nunca lo abre; CI #285). Los runners de GitHub van más
//         lejos: UAC DESHABILITADO, así que NO existe token medio que
//         heredar — ni schtasks /RL LIMITED ni explorer.exe producen uno
//         (runs #289-#290: whoami mostró High Mandatory Level incluso bajo
//         la tarea / el shell).
//      b) El workspace de los runners vive en un VHD montado en D:\a: la
//         MISMA ruta física se resuelve con dos formas (D:\a\... y
//         \Device\HarddiskVolume6\a\...) y Firebird compara las rutas de
//         sus mapeos como STRINGS → "Wrong file for memory mapping:
//         expected ... already mapped ..." (runs #286, #288, #291-#293;
//         con locks únicos por corrida también falló #293, porque el
//         propio seed mapea fb50_trace con ambas formas dentro del VHD).
//         En un volumen REAL ambas formas coinciden: por eso el humo
//         siempre pasó en dev.
//      c) SOLUCIÓN: TODO el humo (seed_ci + app) vía `runas
//         /trustlevel:0x20000` — MISMO token de integridad media con SIDs
//         restringidos para ambos (el mapeo de locks de Firebird embedded
//         no sobrevive a tokens distintos en ninguna dirección: #286
//         sembró elevado y la app degradada no pudo abrir la BD; #294 al
//         revés) — y TODO lo que Firebird mapea FUERA del VHD de D:\a, en
//         el temp de C: (volumen real: una sola forma de ruta; en el VHD
//         la misma ruta se resuelve como D:\a\... y
//         \Device\HarddiskVolume6\a\... y Firebird, que compara strings,
//         chocaba consigo mismo incluso con locks únicos, #293). Everyone
//         concedido (icacls *S-1-1-0:(OI)(CI)F) en resources/firebird y
//         la raíz del repo: los abre la app restringida.
//      d) runas retorna de inmediato: no se puede esperar al hijo por PID;
//         el batch deja marcas de fase (APP-EXIT) al final de su log y el
//         orquestador las sondea. Cada fase registra además una huella
//         compacta de `whoami /groups` para diagnosticar el token real.
//
//   4. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   5. Limpia procesos (vite por árbol, app/seed por imagen, CDP por
//      puerto) y reporta si quedaron residuos del data_dir (Windows puede
//      sostener el .fdb unos segundos tras matar el proceso).
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import {
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	rmSync,
	writeFileSync
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const MANTENER = process.argv.includes('--mantener');
const RAIZ = resolve(import.meta.dirname, '..');
const PUERTO_CDP = process.env.CDP_PORT || '9222';
const EXE_APP = join(RAIZ, 'src-tauri', 'target', 'debug', 'dinamo-rent.exe');
const EXE_SEED = join(RAIZ, 'src-tauri', 'target', 'debug', 'seed_ci.exe');
const BAT_HUMO = join(RAIZ, 'scripts', '.tmp-smoke-humo.cmd');
const LOG_HUMO = join(RAIZ, 'scripts', '.tmp-smoke-humo.log');
const FIREBIRD_RES = join(RAIZ, 'src-tauri', 'resources', 'firebird');
// SID de Todos/Everyone (S-1-1-0): independiente del idioma del SO.
const GRANT_TODOS = '*S-1-1-0:(OI)(CI)F';

// ── Ubicación del humo FUERA del volumen del checkout ──
// En los runners de GitHub el workspace vive en un VHD montado en D:\a: la
// MISMA ruta física se resuelve con dos formas distintas (D:\a\... y
// \Device\HarddiskVolume6\a\...) y Firebird compara las rutas de sus
// mapeos como STRINGS → "Wrong file for memory mapping: expected ...
// already mapped ..." (runs #286, #288, #291-#293; el error salió incluso
// con locks únicos por corrida, porque el propio seed mapea fb50_trace con
// ambas formas dentro del VHD). En un volumen REAL (el disco local) ambas
// formas coinciden y todo funciona — por eso el humo siempre pasó en dev.
// Solución: data_dir (BD + config + locks + temp de Firebird) en el
// temporal del usuario (C:), único por corrida para que además ningún
// mapeo de una corrida anterior choque con el actual.
const FB_RUN_ID = `${Date.now()}_${process.pid}`;
const DATA_DIR = join(tmpdir(), `dinamo-smoke-${FB_RUN_ID}`);
const FDB = join(DATA_DIR, 'dinamo_rent_v3.fdb');
const FB_LOCK_DIR = join(DATA_DIR, 'fblock');
const FB_TMP_DIR = join(DATA_DIR, 'fbtmp');
const FIREBIRD_LOG = join(FIREBIRD_RES, 'firebird.log');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** true si el proceso actual corre con integridad alta (elevado). Solo
 *  informativo: en Windows el humo corre SIEMPRE vía runas. */
function esElevado() {
	if (process.platform !== 'win32') return false;
	const r = spawnSync('whoami', ['/groups'], { encoding: 'utf8' });
	return /S-1-16-12288/.test(r.stdout || '');
}

function matarArbol(pid) {
	// taskkill /T mata el árbol (npm → vite, etc.), /F forzado.
	spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
}

/** true si algún proceso escucha en el puerto dado (netstat). */
function puertoEscuchando(puerto) {
	const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
	return (r.stdout || '')
		.split('\n')
		.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
}

/** Mata los procesos que escuchan en el puerto dado (limpieza defensiva:
 *  la app lanzada vía runas escapa al PID del orquestador). */
async function matarPuerto(puerto) {
	const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
	const pids = new Set();
	for (const l of (r.stdout || '').split('\n')) {
		if (l.includes(`:${puerto} `) && l.includes('LISTENING')) {
			const pid = l.trim().split(/\s+/).pop();
			if (/^\d+$/.test(pid) && pid !== '0') pids.add(pid);
		}
	}
	for (const pid of pids) spawnSync('taskkill', ['/PID', pid, '/T', '/F'], { stdio: 'ignore' });
}

async function esperarPuertoLibre(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (!puertoEscuchando(puerto)) return true;
		await sleep(500);
	}
	return false;
}

/** Espera a que el puerto dado esté en LISTENING (polling 1 s). */
async function esperarPuertoOcupado(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (puertoEscuchando(puerto)) return true;
		await sleep(1000);
	}
	return false;
}

/** Espera a que el CDP suba. null = arriba; 'app-murio' = el batch escribió
 *  APP-EXIT (la app murió antes de abrir el puerto); 'timeout' = venció. */
async function esperarCdp(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (puertoEscuchando(puerto)) return null;
		if (/APP-EXIT\s*$/.test(colaLog(LOG_HUMO, 200))) return 'app-murio';
		await sleep(1000);
	}
	return 'timeout';
}

/** Cola de un archivo de log para adjuntar al diagnóstico de fallo. */
function colaLog(ruta, n = 1500) {
	try {
		return readFileSync(ruta, 'utf8').slice(-n);
	} catch {
		return '(sin log en ' + ruta + ')';
	}
}

/** Borra los data_dirs de corridas anteriores en el temp (si un proceso
 *  fue matado a lo bruto pueden quedar residuos con mapeos
 *  conflictivos). */
function limpiarLocksStale() {
	try {
		for (const entry of readdirSync(tmpdir())) {
			if (entry.startsWith('dinamo-smoke-')) {
				try {
					rmSync(join(tmpdir(), entry), { recursive: true, force: true });
				} catch {
					/* noop: el run id único de esta corrida evita el conflicto. */
				}
			}
		}
	} catch {
		/* noop */
	}
}

/** Concede Full Control a Everyone (con herencia a lo nuevo) sobre las
 *  rutas que la APP degradada debe abrir: los recursos embebidos de
 *  Firebird (firebird.msg, ICU, tzdata y el firebird.log) y la raíz del
 *  repo (CWD del humo, .tmp-print). El data_dir NO se toca: vive en el
 *  temp de C: y lo crea el seed con DACLs por defecto (Usuarios
 *  autentificados ya tienen acceso). Best-effort: si icacls falla en
 *  algún archivo puntual se continúa (el fallo real aparecería al
 *  abrirlo). */
function endurecerAcls() {
	console.log('— concediendo Full Control a Everyone en recursos Firebird y raíz del repo…');
	for (const ruta of [FIREBIRD_RES, RAIZ]) {
		const r = spawnSync(
			'icacls',
			[ruta, '/grant', GRANT_TODOS, '/t', '/c', '/q'],
			{ encoding: 'utf8' }
		);
		if (r.status !== 0) {
			console.warn('⚠ icacls terminó con errores en ' + ruta + ':\n' + (r.stderr || r.stdout || ''));
		}
	}
}

/** Escribe el batch del humo para la fase dada. Fija el env del humo (el
 *  runas NO hereda el entorno del orquestador), registra el token real de
 *  la fase (whoami /groups), redirige todo al log y deja marcas de corte
 *  de fase (SEED-EXIT / APP-EXIT) que el orquestador sondea. */
function escribirBatchHumo(fase) {
	const lineas = [
		'@echo off',
		`set "DINAMO_DATA_DIR=${DATA_DIR}"`,
		`set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=${PUERTO_CDP}"`,
		// Locks/temp/mensajes de Firebird dentro del data_dir (temp de C:,
		// volumen real): la tabla de locks es un mapeo respaldado en ARCHIVO
		// y su ubicación por defecto va bajo el root de Firebird.
		`set "FIREBIRD_LOCK=${FB_LOCK_DIR}"`,
		`set "FIREBIRD_TMP=${FB_TMP_DIR}"`,
		`set "FIREBIRD_MSG=${join(FIREBIRD_RES, 'firebird.msg')}"`,
		`if not exist "${FB_LOCK_DIR}" md "${FB_LOCK_DIR}"`,
		`if not exist "${FB_TMP_DIR}" md "${FB_TMP_DIR}"`,
		`cd /d "${RAIZ}"`,
		// Huella compacta del token: etiqueta de integridad y cómo quedaron
		// Administrators/Usuarios (CSV filtra sin depender del idioma).
		`echo === TOKEN-DE-LA-FASE === >> "${LOG_HUMO}" 2>&1`,
		`powershell -NoProfile -Command "whoami /groups /fo csv | Select-String 'S-1-16-,|S-1-5-32-544|S-1-5-32-545' >> '${LOG_HUMO}'"`
	];
	if (fase === 'seed') {
		lineas.push(
			`"${EXE_SEED}" "${DATA_DIR}" >> "${LOG_HUMO}" 2>&1`,
			'set "EC=%ERRORLEVEL%"',
			`>> "${LOG_HUMO}" echo SEED-EXIT:%EC%`
		);
	} else {
		lineas.push(`"${EXE_APP}" >> "${LOG_HUMO}" 2>&1`, `>> "${LOG_HUMO}" echo APP-EXIT`);
	}
	writeFileSync(BAT_HUMO, lineas.join('\r\n'));
}

/** Lanza el batch vía runas degradado (token de integridad media con SIDs
 *  restringidos: WebView2 honra la bandera CDP y Firebird puede mapear
 *  gracias al grant de Everyone). runas retorna de inmediato: el
 *  orquestador espera por las marcas del log, no por PID. */
function lanzarDegradado() {
	const r = spawn('runas', ['/trustlevel:0x20000', `cmd.exe /c "${BAT_HUMO}"`], {
		cwd: RAIZ,
		stdio: 'ignore'
	});
	if (!r.pid) throw new Error('no se pudo lanzar runas');
}

/** Espera la marca SEED-EXIT del batch degradado (polling 1 s). */
async function esperarMarcaSeed(ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const m = /\r?\nSEED-EXIT:(\d+)\s*$/.exec(colaLog(LOG_HUMO, 200));
		if (m) return Number(m[1]);
		await sleep(1000);
	}
	spawnSync('taskkill', ['/IM', 'seed_ci.exe', '/F'], { stdio: 'ignore' });
	throw new Error('seed_ci (fase humo) no terminó en 5 min:\n' + colaLog(LOG_HUMO));
}

/** Levanta vite como hijo del orquestador: su salida sirve de diagnóstico y
 *  muere con taskkill /T. El dev server corre con el token del orquestador
 *  (elevado en CI): no toca Firebird ni WebView2, y degradarlo rompía la
 *  compilación por los ACE de Administrators (CI #273-#277). */
async function levantarVite() {
	console.log('— lanzando vite (dev server)…');
	const vite = spawn('cmd.exe', ['/d', '/s', '/c', 'npm run dev'], {
		cwd: RAIZ,
		stdio: ['ignore', 'pipe', 'pipe'],
		env: { ...process.env }
	});
	let salida = '';
	vite.stdout.on('data', (d) => (salida += d));
	vite.stderr.on('data', (d) => (salida += d));
	const fin = () => {
		try {
			if (vite.pid) matarArbol(vite.pid);
		} catch {
			/* noop */
		}
	};
	if (!(await esperarPuertoOcupado('5173', 120000))) {
		fin();
		throw new Error('vite (5173) no subió en 2 min:\n' + salida.slice(-1500));
	}
	return { fin, cola: () => salida.slice(-1000) };
}

/** Ejecuta smoke-test-app.mjs contra el CDP ya arriba. */
function correrSmoke() {
	return new Promise((resolveP, rejectP) => {
		const r = spawnSync('node', [join(RAIZ, 'scripts', 'smoke-test-app.mjs')], {
			cwd: RAIZ,
			stdio: 'inherit',
			env: { ...process.env, CDP_PORT: PUERTO_CDP }
		});
		if (r.status !== 0) rejectP(new Error(`smoke falló (exit ${r.status})`));
		else resolveP();
	});
}

async function main() {
	console.log('== smoke:dev — flujo completo con BD aislada y vacía ==');
	// Temporales de corridas abortadas fuera del camino.
	rmSync(BAT_HUMO, { force: true });
	rmSync(LOG_HUMO, { force: true });

	const esWin = process.platform === 'win32';

	// ── Rama no-Windows: sin concepto de elevación, `tauri dev` completo. ──
	if (!esWin) {
		console.log('— sembrando BD aislada (seed_ci)…');
		const rs = spawnSync('cargo', ['run', '--features', 'dev', '--bin', 'seed_ci', '--', DATA_DIR], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rs.status !== 0) throw new Error('seed_ci falló');
		if (existsSync(FDB)) console.log('   BD aislada:', FDB);

		// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a
		// vite a recrearlo con su propia propiedad; en local es un directorio
		// generado, sin costo.
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		console.log('— lanzando tauri dev (BD aislada + CDP ' + PUERTO_CDP + ')…');
		const dev = spawn('npm', ['run', 'tauri', 'dev'], {
			cwd: RAIZ,
			stdio: ['ignore', 'pipe', 'pipe'],
			env: {
				...process.env,
				DINAMO_DATA_DIR: DATA_DIR,
				WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP}`
			}
		});
		let salida = '';
		dev.stdout.on('data', (d) => (salida += d));
		dev.stderr.on('data', (d) => (salida += d));

		try {
			if (!(await esperarPuertoOcupado(PUERTO_CDP, 420000))) {
				throw new Error(`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` + salida.slice(-1500));
			}
			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} finally {
			try {
				if (dev.pid) matarArbol(dev.pid);
			} catch {
				/* noop */
			}
		}
	} else {
		// ── Rama Windows (CI elevado y dev local): un solo camino. ──
		const elevado = esElevado();

		// Compilar con el token del orquestador: seed_ci tiene
		// required-features=["dev"] (feature vacío que solo lo excluye del
		// bundle de release) — sin --features dev el build lo salta y el exe
		// no existe (corrida 287). --bins acota el build a los binarios.
		console.log('— compilando app + seed_ci (cargo build --features dev --bins)…');
		const rc = spawnSync('cargo', ['build', '--features', 'dev', '--bins'], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rc.status !== 0) throw new Error(`cargo build falló (exit ${rc.status})`);
		if (!existsSync(EXE_APP)) throw new Error('no existe el binario compilado: ' + EXE_APP);
		if (!existsSync(EXE_SEED)) throw new Error('no existe el binario seed_ci: ' + EXE_SEED);

		// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a
		// vite a recrearlo con su propia propiedad (crítico si el checkout lo
		// creó con otro token).
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		// Corridas anteriores abortadas fuera del temp (mapeos conflictivos).
		limpiarLocksStale();
		// Recursos Firebird y raíz del repo con Everyone: los abre la app
		// degradada (el seed elevado no lo necesita).
		endurecerAcls();

		// ── Semilla (vía runas: MISMO token restringido que la app — el
		// mapeo de locks de Firebird embedded no sobrevive a tokens distintos
		// en ninguna dirección, runs #286 y #294) sobre el data_dir del temp
		// de C: — volumen REAL, fuera del VHD de D:\a donde la misma ruta se
		// resuelve con dos formas (unidad vs kernel) y Firebird, que compara
		// strings, chocaba consigo mismo (#291-#293, incluso con locks
		// únicos). El run id único por corrida evita además choques con
		// mapeos de corridas abortadas.
		console.log('— sembrando BD aislada (seed_ci vía runas, data_dir en temp de C:)…');
		mkdirSync(DATA_DIR, { recursive: true });
		rmSync(LOG_HUMO, { force: true });
		escribirBatchHumo('seed');
		lanzarDegradado();
		const code = await esperarMarcaSeed(300000);
		if (code !== 0) {
			throw new Error(
				`seed_ci falló (exit ${code}):\n${colaLog(LOG_HUMO)}\n` +
					`\n--- firebird.log ---\n${colaLog(FIREBIRD_LOG, 2000)}`
			);
		}
		if (!existsSync(FDB)) throw new Error('seed_ci no produjo la BD: ' + FDB);
		console.log('   BD aislada:', FDB);

		// ── Vite (token del orquestador) ──
		const vite = await levantarVite();

		const finApp = () => {
			// La app puede sobrevivir a su árbol (runas suelta el PID):
			// matarla por nombre (target/debug solo existe en desarrollo).
			spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
		};

		// ── App (degradada: WebView2 solo honra la bandera CDP con
		// integridad media; el token del runas la tiene). Abre la BD creada
		// por el seed elevado SOLO tras los grants de Everyone (#286 falló
		// justo porque sembraba elevado sin conceder nada). ──
		console.log(
			'— lanzando la app (' +
				(elevado ? 'orquestador elevado' : 'orquestador sin elevar') +
				', app vía runas, CDP ' +
				PUERTO_CDP +
				')…'
		);
		rmSync(LOG_HUMO, { force: true }); // log limpio para la fase app
		escribirBatchHumo('app');
		lanzarDegradado();

		try {
			const razon = await esperarCdp(PUERTO_CDP, 420000);
			if (razon === 'timeout') {
				throw new Error(
					`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` +
						`--- log del humo ---\n${colaLog(LOG_HUMO)}\n` +
						`--- cola del dev server ---\n${vite.cola()}`
				);
			}
			if (razon === 'app-murio') {
				throw new Error('la app murió antes de abrir CDP:\n' + colaLog(LOG_HUMO));
			}
			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} finally {
			finApp();
			await matarPuerto(PUERTO_CDP);
			await matarPuerto('5173');
			await esperarPuertoLibre(PUERTO_CDP, 10000);
			await esperarPuertoLibre('5173', 10000);
			vite.fin();
		}
	}

	// Limpieza defensiva por imagen: seed_ci puede quedar colgado si la
	// espera por marca venció, y el .fdb quedaría retenido.
	if (esWin) {
		spawnSync('taskkill', ['/IM', 'seed_ci.exe', '/F'], { stdio: 'ignore' });
	}

	if (MANTENER) {
		console.log('--mantener: data_dir conservado en ' + DATA_DIR);
	} else {
		rmSync(DATA_DIR, { recursive: true, force: true });
		if (!existsSync(DATA_DIR)) console.log('✓ limpieza: data_dir aislado eliminado');
		else console.log('⚠ el data_dir quedó bloqueado (BORRAR A MANO): ' + DATA_DIR);
	}
	rmSync(BAT_HUMO, { force: true });
	rmSync(LOG_HUMO, { force: true });
	console.log('LISTO — smoke:dev OK');
}

main()
	.then(() => process.exit(0))
	.catch((e) => {
		console.error('✗ FALLO smoke:dev:', e.message);
		process.exit(1);
	});
