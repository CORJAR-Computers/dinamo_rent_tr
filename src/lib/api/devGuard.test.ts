// src/lib/api/devGuard.test.ts — Tests del guardrail de desarrollo
import { describe, it, expect, vi, beforeEach, afterEach, type MockInstance } from 'vitest';
import { devTypeCheckArgs } from './devGuard';

describe('devTypeCheckArgs (guardrail de desarrollo)', () => {
	let warn: MockInstance<typeof console.warn>;

	beforeEach(() => {
		// En vitest DEV ya es true, pero lo fijamos explícito para independencia
		vi.stubEnv('DEV', true);
		warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
	});

	afterEach(() => {
		warn.mockRestore();
		vi.unstubAllEnvs();
	});

	it('avisa cuando un campo string del DTO viaja como number (incidente extender_renta)', () => {
		devTypeCheckArgs('extender_renta', {
			sessionId: 's1',
			datos: { tipo: 'horas', cantidad: 1, valor: 25000.5 }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		const msg = String(warn.mock.calls[0]?.[0]);
		expect(msg).toContain('[devGuard]');
		expect(msg).toContain('extender_renta');
		expect(msg).toContain('"valor"');
		expect(msg).toContain('25000.5');
	});

	it('no avisa cuando el campo viaja como string (contrato correcto)', () => {
		devTypeCheckArgs('extender_renta', {
			sessionId: 's1',
			datos: { tipo: 'horas', cantidad: 2, valor: '25000.5' }
		});

		expect(warn).not.toHaveBeenCalled();
	});

	it('avisa por cada campo monetario incorrecto en el mismo payload', () => {
		devTypeCheckArgs('registrar_pago_renta', {
			sessionId: 's1',
			idRenta: 3,
			datos: { monto: 100000, metodoPago: 'Efectivo', concepto: 'Abono' }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		expect(String(warn.mock.calls[0]?.[0])).toContain('"monto"');
	});

	it('cubre el alias actualizar_* (mismo contrato que crear_*)', () => {
		devTypeCheckArgs('actualizar_gasto', {
			sessionId: 's1',
			id: 7,
			datos: { fecha: '2026-09-16', categoria: 'Combustible', descripcion: 'x', monto: 999 }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		expect(String(warn.mock.calls[0]?.[0])).toContain('"monto"');
	});

	it('no avisa con payload correcto multi-campo (crear_renta)', () => {
		devTypeCheckArgs('crear_renta', {
			sessionId: 's1',
			datos: {
				valorDia: '150000',
				total: '750000',
				kmSalida: '42000',
				// Los campos i64/bool NO están en el mapa: el número es su tipo correcto
				diasCalculados: 5,
				cobraIva: false
			}
		});

		expect(warn).not.toHaveBeenCalled();
	});

	it('reporta cada uno de los varios campos numéricos detectados', () => {
		devTypeCheckArgs('editar_renta_cerrada', {
			sessionId: 's1',
			id: 9,
			datos: { valorDia: 180000, descuento: 5000, observaciones: 'corrección' }
		});

		expect(warn).toHaveBeenCalledTimes(2);
		const mensajes = warn.mock.calls.map((c) => String(c[0]));
		expect(mensajes.some((m) => m.includes('"valorDia"'))).toBe(true);
		expect(mensajes.some((m) => m.includes('"descuento"'))).toBe(true);
	});

	it('es inoperante fuera de DEV (producción silenciosa)', () => {
		vi.stubEnv('DEV', false);
		devTypeCheckArgs('extender_renta', {
			datos: { tipo: 'horas', cantidad: 1, valor: 150000 }
		});

		expect(warn).not.toHaveBeenCalled();
	});

	it('ignora payloads sin datos, con datos no-objeto o comandos no mapeados', () => {
		devTypeCheckArgs('extender_renta', { sessionId: 's1' });
		devTypeCheckArgs('extender_renta');
		devTypeCheckArgs('extender_renta', { datos: 'no-soy-objeto' });
		devTypeCheckArgs('extender_renta', { datos: [150000] });
		devTypeCheckArgs('comando_desconocido', { datos: { valor: 150000 } });

		expect(warn).not.toHaveBeenCalled();
	});

	it('nunca muta el payload ni lanza', () => {
		const datos = { tipo: 'horas', cantidad: 1, valor: 150000 };
		const args = { sessionId: 's1', datos };

		expect(() => devTypeCheckArgs('extender_renta', args)).not.toThrow();
		expect(datos.valor).toBe(150000);
		expect(typeof datos.valor).toBe('number');
	});

	// ——— Dirección inversa: campos i64/f64 que viajan como string ———

	it('avisa cuando un campo i64 viaja como string (cantidad de extensión)', () => {
		devTypeCheckArgs('extender_renta', {
			sessionId: 's1',
			datos: { tipo: 'horas', cantidad: '2', valor: '25000' }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		const msg = String(warn.mock.calls[0]?.[0]);
		expect(msg).toContain('"cantidad"');
		expect(msg).toContain('declara i64');
		expect(msg).toContain('invalid type: string, expected i64');
	});

	it('no avisa cuando los campos numéricos viajan como number o null (contrato correcto)', () => {
		devTypeCheckArgs('crear_renta', {
			sessionId: 's1',
			datos: {
				idCliente: 3,
				diasCalculados: 5,
				horasExtras: 0,
				idReserva: null,
				valorDia: '150000'
			}
		});

		expect(warn).not.toHaveBeenCalled();
	});

	it('detecta la cadena vacía en campo i64 (el caso que serde rechaza seguro)', () => {
		devTypeCheckArgs('crear_reserva', {
			sessionId: 's1',
			datos: { diasCalculados: '', horasExtras: 0, valorDia: '150000' }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		expect(String(warn.mock.calls[0]?.[0])).toContain('"diasCalculados"');
	});

	it('cubre el alias actualizar_* en la dirección inversa', () => {
		devTypeCheckArgs('actualizar_auto', {
			sessionId: 's1',
			placa: 'ABC123',
			datos: { kilometraje: '42000', costoFijoMensual: '300000' }
		});

		expect(warn).toHaveBeenCalledTimes(1);
		const msg = String(warn.mock.calls[0]?.[0]);
		expect(msg).toContain('"kilometraje"');
		expect(msg).toContain('declara i64');
	});

	it('reporta ambas direcciones en el mismo payload mixto', () => {
		devTypeCheckArgs('crear_renta', {
			sessionId: 's1',
			datos: {
				valorDia: 150000,
				diasCalculados: '5',
				horasExtras: 0
			}
		});

		expect(warn).toHaveBeenCalledTimes(2);
		const mensajes = warn.mock.calls.map((c) => String(c[0]));
		expect(mensajes.some((m) => m.includes('"valorDia"') && m.includes('declara string'))).toBe(
			true
		);
		expect(mensajes.some((m) => m.includes('"diasCalculados"') && m.includes('declara i64'))).toBe(
			true
		);
	});
});
