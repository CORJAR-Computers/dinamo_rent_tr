#!/usr/bin/env node
// smoke-dev.mjs — Smoke E2E en dev con BD AISLADA y VACÍA de rentas.
//
// A diferencia de `smoke:app` (que corre contra la BD que haya), este
// orquestador garantiza el escenario completo del flujo de cobro:
//
//   1. Crea un data_dir temporal (`scripts/.tmp-smoke-data`, ignorado por git).
//   2. Lo siembra con `seed_ci <dir>`: config.ini + BD con admin/autos/
//      clientes, pero SIN rentas → la tabla de /rentas arranca vacía y el
//      smoke entra al branch de "renta de prueba" (el único que conoce la
//      base monetaria y puede asertar los totales de la extensión).
//   3. Compila la app (`cargo build`), levanta vite (dev server) y lanza el
//      EXE ya compilado con DINAMO_DATA_DIR apuntando al dir temporal y CDP
//      en 9222 (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS).
//
//      ⚠️ POR QUÉ TRES PROCESOS SEPARADOS (en vez de `tauri dev`): en un
//      proceso ELEVADO (los runners de CI), Chromium/WebView2 IGNORA la
//      bandera --remote-debugging-port y el puerto CDP nunca abre — probado
//      localmente con UAC: el mismo exe sin elevar abre 9222 en 1 s y elevado
//      nunca lo abre, con y sin la variable en el entorno (CI #285). Pero
//      degradar TODO el árbol con `runas /trustlevel:0x20000` (CI #273-#277)
//      rompió la compilación: el token restringido pierde los ACE del grupo
//      Administrators → `.cargo-build-lock` denegado y `EPERM` de vite sobre
//      `.svelte-kit`. Solución: compilar y servir ELEVADOS, y degradar el
//      seed_ci y SOLO el proceso de la app (los procesos que tocan Firebird:
//      un mapeo de memoria de la BD creado elevado no es accesible para el
//      token restringido → "Wrong file for memory mapping", CI run #286).
//      Detalle clave: runas NO hereda el entorno del orquestador, así que el
//      env del humo (BD aislada + bandera CDP) se fija DENTRO del batch que
//      se ejecuta ya degradado; el log va a un archivo que este orquestador
//      lee — tanto para el diagnóstico como para esperar el fin del seed
//      (marca EXIT:<code> al final del log, pues runas retorna de inmediato).
//
//   4. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   5. Limpia procesos (vite/app por árbol e imagen, CDP por puerto) y
//      reporta si quedaron residuos del data_dir (Windows puede sostener el
//      .fdb unos segundos tras matar el proceso).
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const MANTENER = process.argv.includes('--mantener');
const RAIZ = resolve(import.meta.dirname, '..');
const DATA_DIR = join(RAIZ, 'scripts', '.tmp-smoke-data');
const FDB = join(DATA_DIR, 'dinamo_rent_v3.fdb');
const PUERTO_CDP = process.env.CDP_PORT || '9222';
const EXE_APP = join(RAIZ, 'src-tauri', 'target', 'debug', 'dinamo-rent.exe');
const EXE_SEED = join(RAIZ, 'src-tauri', 'target', 'debug', 'seed_ci.exe');
const BAT_APP = join(RAIZ, 'scripts', '.tmp-smoke-app.cmd');
const LOG_APP = join(RAIZ, 'scripts', '.tmp-smoke-app.log');
const BAT_SEED = join(RAIZ, 'scripts', '.tmp-smoke-seed.cmd');
const LOG_SEED = join(RAIZ, 'scripts', '.tmp-smoke-seed.log');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** true si el proceso actual corre con integridad alta (elevado). Los runners
 *  de CI de Windows ejecutan elevados y ahí WebView2 ignora --remote-debugging-port. */
function esElevado() {
	if (process.platform !== 'win32') return false;
	const r = spawnSync('whoami', ['/groups'], { encoding: 'utf8' });
	return /S-1-16-12288/.test(r.stdout || '');
}

function matarArbol(pid) {
	// taskkill /T mata el árbol (npm → vite, etc.), /F forzado.
	spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
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
		const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
		const ocupado = (r.stdout || '')
			.split('\n')
			.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
		if (!ocupado) return true;
		await sleep(500);
	}
	return false;
}

/** Espera a que el puerto dado esté en LISTENING (polling 1 s). */
async function esperarPuertoOcupado(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
		const ok = (r.stdout || '')
			.split('\n')
			.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
		if (ok) return true;
		await sleep(1000);
	}
	return false;
}

/** Cola de un archivo de log para adjuntar al diagnóstico de fallo. */
function colaLog(ruta, n = 1500) {
	try {
		return readFileSync(ruta, 'utf8').slice(-n);
	} catch {
		return '(sin log en ' + ruta + ')';
	}
}

/** Lanza un batch DEGRADADO vía runas y espera a que el proceso termine
 *  leyendo el marcador `EXIT:<code>` que el batch escribe al final del log
 *  (runas retorna de inmediato y no deja esperar al hijo por PID). */
async function correrBatchDegradado(bat, log, ms) {
	spawn('runas', ['/trustlevel:0x20000', `cmd.exe /c "${bat}"`], {
		cwd: RAIZ,
		stdio: 'ignore'
	});
	const MARCA = /\r?\nEXIT:(\d+)\s*$/;
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const m = MARCA.exec(colaLog(log, 200));
		if (m) return Number(m[1]);
		await sleep(1000);
	}
	throw new Error(`el proceso degradado no terminó en ${Math.round(ms / 1000)} s:\n--- log ---\n${colaLog(log)}`);
}

/** Escribe un batch que fija el env dado, corre un exe y deja la marca
 *  `EXIT:<code>` al final del log (para correrBatchDegradado). */
function escribirBatch(rutaBat, rutaLog, exe, env, msTimeout) {
	writeFileSync(
		rutaBat,
		[
			'@echo off',
			...Object.entries(env).map(([k, v]) => `set "${k}=${v}"`),
			`cd /d "${RAIZ}"`,
			`"${exe}" >> "${rutaLog}" 2>&1`,
			'set "EC=%ERRORLEVEL%"',
			`>> "${rutaLog}" echo EXIT:%EC%`,
			...(msTimeout ? [`timeout /t ${Math.ceil(msTimeout / 1000)} /nobreak > nul`] : [])
		].join('\r\n')
	);
}

async function main() {
	console.log('== smoke:dev — flujo completo con BD aislada y vacía ==');
	// Temporales de corridas abortadas fuera del camino.
	rmSync(BAT_APP, { force: true });
	rmSync(LOG_APP, { force: true });
	rmSync(BAT_SEED, { force: true });
	rmSync(LOG_SEED, { force: true });

	const esWin = process.platform === 'win32';
	const elevado = esWin ? esElevado() : false;

	// Compilar ANTES de sembrar cuando el seed irá degradado (CI): necesita
	// seed_ci.exe ya compilado — el token restringido no puede escribir en
	// target/. (Sin elevar, el `cargo run` del seed compila por su cuenta.)
	if (esWin && elevado) {
		console.log('— compilando binarios (cargo build, elevado)…');
		const rc = spawnSync('cargo', ['build'], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rc.status !== 0) throw new Error(`cargo build falló (exit ${rc.status})`);
	}

	if (!existsSync(FDB)) {
		console.log('— sembrando BD aislada (seed_ci)…');
		if (esWin && elevado) {
			// El seed corre DEGRADADO (mismo contexto que la app): Firebird
			// embedded mapea la BD y sus locks en memoria compartida y un mapeo
			// creado por el proceso elevado no es accesible para el token
			// restringido de la app degradada → "Wrong file for memory mapping"
			// (CI run #286). Para esperar su fin vía runas, el batch deja una
			// marca EXIT:<code> al final del log.
			if (!existsSync(EXE_SEED)) throw new Error('no existe el binario seed_ci: ' + EXE_SEED);
			escribirBatch(BAT_SEED, LOG_SEED, EXE_SEED, {}, 30000);
			const code = await correrBatchDegradado(BAT_SEED, LOG_SEED, 300000);
			if (code !== 0) throw new Error(`seed_ci falló (exit ${code}):\n${colaLog(LOG_SEED)}`);
		} else {
			const r = spawnSync('cargo', ['run', '--features', 'dev', '--bin', 'seed_ci', '--', DATA_DIR], {
				cwd: join(RAIZ, 'src-tauri'),
				stdio: 'inherit'
			});
			if (r.status !== 0) throw new Error('seed_ci falló');
		}
	} else {
		console.log(
			'— BD aislada ya sembrada (reutilizando; borra scripts/.tmp-smoke-data para renovar)'
		);
	}
	if (existsSync(FDB)) console.log('   BD aislada:', FDB);

	// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a vite
	// a recrearlo con su propia propiedad; en local es un directorio generado,
	// sin costo.
	rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

	const finApp = () => {
		// La app puede sobrevivir a su árbol (runas suelta el PID): matarla por
		// nombre (target/debug solo existe en desarrollo).
		if (esWin) spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
	};

	// ── Rama no-Windows: sin concepto de elevación, `tauri dev` completo. ──
	if (!esWin) {
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
		// ── Rama Windows (CI elevado y dev local): tres procesos separados. ──

		// 1) Compilar ANTES (proceso elevado): el runas sólo lanza el exe ya
		//    compilado — el token restringido no puede escribir en target/
		//    (no-op si ya se compiló para el seed degradado).
		console.log('— compilando la app (cargo build, elevado)…');
		const rb = spawnSync('cargo', ['build'], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rb.status !== 0) throw new Error(`cargo build falló (exit ${rb.status})`);
		if (!existsSync(EXE_APP)) throw new Error('no existe el binario compilado: ' + EXE_APP);

		// 2) Vite elevado, hijo del orquestador: su salida sirve de diagnóstico
		//    y muere con taskkill /T. (Elevado NO tiene el EPERM de antes: ese
		//    lo causaba escribir .svelte-kit desde el proceso DEGRADADO.)
		console.log('— lanzando vite (dev server)…');
		const vite = spawn('cmd.exe', ['/d', '/s', '/c', 'npm run dev'], {
			cwd: RAIZ,
			stdio: ['ignore', 'pipe', 'pipe'],
			env: { ...process.env }
		});
		let salidaVite = '';
		vite.stdout.on('data', (d) => (salidaVite += d));
		vite.stderr.on('data', (d) => (salidaVite += d));
		const finVite = () => {
			try {
				if (vite.pid) matarArbol(vite.pid);
			} catch {
				/* noop */
			}
		};
		if (!(await esperarPuertoOcupado('5173', 120000))) {
			finVite();
			throw new Error('vite (5173) no subió en 2 min:\n' + salidaVite.slice(-1500));
		}

		// 3) Batch con el env del humo (runas no hereda entorno) + lanzamiento
		//    degradado SOLO de la app: es el proceso que debe aceptar la bandera
		//    CDP; compilación y dev server siguen elevados.
		escribirBatch(BAT_APP, LOG_APP, EXE_APP, {
			DINAMO_DATA_DIR: DATA_DIR,
			WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP}`
		});
		console.log(
			'— lanzando la app (' +
				(elevado ? 'degradada vía runas: solo el proceso de la app' : 'directa') +
				', CDP ' +
				PUERTO_CDP +
				')…'
		);
		if (elevado) {
			// runas retorna de inmediato: el batch queda vivo por su cuenta y el
			// log de la app se captura en archivo (no por pipe).
			spawn('runas', ['/trustlevel:0x20000', `cmd.exe /c "${BAT_APP}"`], {
				cwd: RAIZ,
				stdio: 'ignore'
			});
		} else {
			spawn('cmd.exe', ['/d', '/s', '/c', BAT_APP], { cwd: RAIZ, stdio: 'ignore' });
		}

		try {
			// CDP arriba = la ventana WebView2 de la app ya existe.
			if (!(await esperarPuertoOcupado(PUERTO_CDP, 420000))) {
				throw new Error(
					`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` +
						`--- log de la app ---\n${colaLog(LOG_APP)}\n` +
						`--- cola del dev server ---\n${salidaVite.slice(-1000)}`
				);
			}
			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} finally {
			finApp();
			await matarPuerto(PUERTO_CDP);
			await matarPuerto('5173');
			await esperarPuertoLibre(PUERTO_CDP, 10000);
			await esperarPuertoLibre('5173', 10000);
			finVite();
		}
	}

	if (MANTENER) {
		console.log('--mantener: data_dir conservado en ' + DATA_DIR);
	} else {
		rmSync(DATA_DIR, { recursive: true, force: true });
		if (!existsSync(DATA_DIR)) console.log('✓ limpieza: data_dir aislado eliminado');
		else console.log('⚠ el data_dir quedó bloqueado (BORRAR A MANO): ' + DATA_DIR);
	}
	rmSync(BAT_APP, { force: true });
	rmSync(LOG_APP, { force: true });
	rmSync(BAT_SEED, { force: true });
	rmSync(LOG_SEED, { force: true });
	console.log('LISTO — smoke:dev OK');
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

main()
	.then(() => process.exit(0))
	.catch((e) => {
		console.error('✗ FALLO smoke:dev:', e.message);
		process.exit(1);
	});
