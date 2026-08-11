#!/usr/bin/env node
// scripts/test-check-simit.mjs — Test del flujo 401 → sonda E2E de check-simit
//
// Levanta un servidor HTTP local que simula el portal SIMIT (mismo contrato
// que el gateway real desde el 11-08: 401 selectivo por token):
//
//   GET  /          → 503 (página principal — advertencia, no bloquea)
//   POST /api.php   → {error:false, data:{question, recommended_difficulty:2}}
//   POST /consulta  → 401 si el token es '[]' (probe SIN token)
//                     200 con multas si trae un token real (sonda E2E)
//
// Ejecuta `node scripts/check-simit.mjs --json` apuntando las URLs al servidor
// local (SIMIT_CAPTCHA_URL / SIMIT_CONSULTA_URL / SIMIT_PAGINA_URL +
// SIMIT_DNS_SKIP=1) y verifica:
//   1. Ante el 401 del probe sin token se dispara la sonda E2E con token real.
//   2. El token de la sonda es de UNA solución (Fase 1 / solo_primera_solucion).
//   3. El veredicto final es `operativo` (exit 0).
//   4. Escenario negativo: si el 401 persiste CON token real → `micro_caido`
//      (firma gateway, exit 2), no un falso positivo.
//
// Uso: node scripts/test-check-simit.mjs   (npm run test:check-simit)

import { spawn } from 'node:child_process';
import http from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CHECK = join(__dirname, 'check-simit.mjs');

const fallos = [];
const ok = (m) => console.log(`  ✓ ${m}`);
const falla = (m) => {
	console.log(`  ✗ ${m}`);
	fallos.push(m);
};

/** Servidor que imita el contrato del gateway (401 selectivo por token). */
async function levantarServidor({ siempre401 = false } = {}) {
	const conteo = { captchas: 0, consultas: [] }; // consultas: [{token, status}]
	const server = http.createServer((req, res) => {
		let body = '';
		req.on('data', (d) => (body += d));
		req.on('end', () => {
			const url = new URL(req.url, 'http://localhost');
			res.setHeader('Content-Type', 'application/json');
			if (req.method === 'GET' && url.pathname === '/') {
				res.writeHead(503);
				res.end('<!DOCTYPE html><title>sim</title>Server-unavailable!');
				return;
			}
			if (req.method === 'POST' && url.pathname === '/api.php') {
				conteo.captchas += 1;
				res.writeHead(200);
				res.end(
					JSON.stringify({
						error: false,
						data: { question: 'test-q-simit', recommended_difficulty: 2 }
					})
				);
				return;
			}
			if (req.method === 'POST' && url.pathname === '/consulta') {
				let token = '';
				try {
					token = JSON.parse(body)?.reCaptchaDTO?.response ?? '';
				} catch {
					/* body no JSON: sin token */
				}
				const tokenValido = !siempre401 && token !== '[]' && token.length > 0;
				conteo.consultas.push({ token, status: tokenValido ? 200 : 401 });
				if (tokenValido) {
					res.writeHead(200);
					res.end(
						JSON.stringify({
							multas: [
								{
									comparendo: true,
									numeroComparendo: 'TEST-250010000000001',
									valorPagar: 137300,
									estadoComparendo: 'PENDIENTE',
									fechaComparendo: '11/08/2026',
									organismoTransito: 'Pruebas',
									infracciones: [
										{ codigoInfraccion: 'C23', descripcionInfraccion: 'Multa de prueba' }
									]
								}
							],
							pazSalvo: false,
							cancelada: false,
							suspendida: false
						})
					);
				} else {
					res.writeHead(401);
					res.end(
						JSON.stringify({
							codigo: 5,
							descripcion: 'Autenticación fallida: Acceso denegado'
						})
					);
				}
				return;
			}
			res.writeHead(404);
			res.end('{}');
		});
	});
	await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
	const { port } = server.address();
	return { base: `http://127.0.0.1:${port}`, conteo, server };
}

/** Ejecuta check-simit.mjs --json con las URLs apuntando al servidor local. */
function ejecutarCheck(env, args = []) {
	return new Promise((resolve) => {
		const child = spawn(process.execPath, [CHECK, '--json', ...args], {
			env: { ...process.env, ...env },
			stdio: ['ignore', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (d) => (stdout += d));
		child.stderr.on('data', (d) => (stderr += d));
		child.on('close', (codigo) =>
			resolve({ codigo: codigo ?? 1, stdout, stderr: stderr.trim() })
		);
	});
}

function envLocal(s) {
	return {
		SIMIT_CAPTCHA_URL: `${s.base}/api.php`,
		SIMIT_CONSULTA_URL: `${s.base}/consulta`,
		SIMIT_PAGINA_URL: `${s.base}/`,
		SIMIT_DNS_SKIP: '1'
	};
}

// ─── Escenario 1: gateway selectivo (401 sin token → 200 con token real) ─────
console.log('\nEscenario 1 — gateway selectivo por token (esperado: operativo)');
const s1 = await levantarServidor();
try {
	// --multas: además del veredicto, el JSON debe traer el resumen E2E
	const r = await ejecutarCheck(envLocal(s1), ['--multas']);
	let reporte = null;
	try {
		reporte = JSON.parse(r.stdout);
	} catch {
		/* salida no JSON */
	}
	console.log(`  exit=${r.codigo} · resultado=${reporte?.resultado} · firma=${reporte?.microservicio?.firma}${r.stderr ? ` · stderr: ${r.stderr.slice(0, 100)}` : ''}`);

	if (r.codigo === 0) ok('exit 0');
	else falla(`exit ${r.codigo} (esperado 0) — stderr: ${r.stderr.slice(0, 200)}`);
	if (reporte?.resultado === 'operativo') ok('resultado = operativo');
	else falla(`resultado = ${reporte?.resultado} (esperado operativo)`);
	if (reporte?.microservicio?.firma === 'up') ok('firma del micro = up');
	else falla(`firma = ${reporte?.microservicio?.firma} (esperado up)`);

	// El 401 del probe sin token debe disparar la sonda E2E (2ª consulta con token)
	const n = s1.conteo.consultas.length;
	if (n >= 2) ok(`la sonda E2E se disparó: ${n} consultas al endpoint (probe + E2E)`);
	else falla(`solo ${n} consulta(s); se esperaba ≥2 (el 401 sin token debía disparar la E2E)`);
	if (s1.conteo.consultas[0]?.status === 401) ok('1ª consulta (sin token) → 401');
	else falla(`1ª consulta status = ${s1.conteo.consultas[0]?.status} (esperado 401)`);
	const ultima = s1.conteo.consultas[n - 1];
	if (ultima?.status === 200) ok('última consulta (con token real) → 200');
	else falla(`última consulta status = ${ultima?.status} (esperado 200)`);
	if (s1.conteo.captchas >= 1) ok(`captchas resueltos: ${s1.conteo.captchas}`);
	else falla('no se resolvió ningún captcha para la sonda E2E');

	// Token de UNA solución (Fase 1): el array JSON trae exactamente 1 nonce
	const nonces = (ultima?.token.match(/"nonce"/g) ?? []).length;
	if (nonces === 1) ok('token de la sonda = UNA solución (solo_primera_solucion)');
	else falla(`token con ${nonces} solución(es) (esperado 1) — ${String(ultima?.token).slice(0, 60)}`);

	// --multas: el resumen E2E debe viajar en el JSON (microservicio.e2e)
	const e2e = reporte?.microservicio?.e2e;
	if (e2e?.multas === 1 && e2e?.totalPendiente === 137300) {
		ok(`e2e en el JSON: ${e2e.multas} multa(s) · $${e2e.totalPendiente} (con --multas)`);
	} else {
		falla(`e2e ausente o incorrecto (esperado multas=1, totalPendiente=137300) — ${JSON.stringify(e2e)}`);
	}
	if (e2e?.detalles?.[0]?.numero === 'TEST-250010000000001') {
		ok(`detalle por multa en e2e.detalles (${e2e.detalles.length} fila)`);
	} else {
		falla(`e2e.detalles no coincide — ${JSON.stringify(e2e?.detalles?.[0])}`);
	}
} finally {
	await new Promise((resolve) => s1.server.close(resolve));
}

// ─── Escenario 2: gateway bloquea hasta con token real (esperado: caído) ─────
console.log('\nEscenario 2 — 401 persiste con token real (esperado: micro_caido)');
const s2 = await levantarServidor({ siempre401: true });
try {
	const r = await ejecutarCheck(envLocal(s2));
	let reporte = null;
	try {
		reporte = JSON.parse(r.stdout);
	} catch {
		/* salida no JSON */
	}
	console.log(`  exit=${r.codigo} · resultado=${reporte?.resultado} · firma=${reporte?.microservicio?.firma}`);

	if (reporte?.resultado === 'micro_caido') ok('resultado = micro_caido');
	else falla(`resultado = ${reporte?.resultado} (esperado micro_caido)`);
	if (reporte?.microservicio?.firma === 'gateway') ok('firma del micro = gateway');
	else falla(`firma = ${reporte?.microservicio?.firma} (esperado gateway)`);
	if (r.codigo === 2) ok('exit 2 (SIMIT caído)');
	else falla(`exit ${r.codigo} (esperado 2)`);
	if (s2.conteo.consultas.length >= 2) ok('también se disparó la sonda E2E (y confirmó la caída)');
	else falla(`solo ${s2.conteo.consultas.length} consulta(s) — se esperaba probe + E2E`);
} finally {
	await new Promise((resolve) => s2.server.close(resolve));
}

// ─── Resumen ─────────────────────────────────────────────────────────────────
console.log('');
if (fallos.length === 0) {
	console.log('✅ test-check-simit: TODOS LOS ESCENARIOS PASAN');
	process.exitCode = 0;
} else {
	console.error(`✗ test-check-simit: ${fallos.length} fallo(s)`);
	process.exitCode = 1;
}
