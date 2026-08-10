// src/lib/utils/format.test.ts — Tests del formateador COP y fechas
import { describe, it, expect } from 'vitest';
import { formatCOP, formatContrato, formatDate, formatDateTime, truncate } from './format';

/**
 * Normaliza el espacio no separable (U+00A0) que Intl es-CO inserta
 * entre el símbolo y el monto (p. ej. "$ 1.500.000") para poder
 * comparar strings de forma legible.
 */
const norm = (s: string) => s.replace(/\u00A0/g, ' ');

describe('formatCOP', () => {
	it('formatea montos enteros en pesos colombianos', () => {
		expect(norm(formatCOP(1500000))).toBe('$ 1.500.000');
		expect(norm(formatCOP(0))).toBe('$ 0');
		expect(norm(formatCOP(1234567))).toBe('$ 1.234.567');
	});

	it('acepta strings numéricos', () => {
		expect(norm(formatCOP('490000'))).toBe('$ 490.000');
		expect(norm(formatCOP('1500000.75'))).toBe('$ 1.500.001');
	});

	it('devuelve $0 para valores nulos o no numéricos', () => {
		expect(formatCOP(null)).toBe('$0');
		expect(formatCOP(undefined)).toBe('$0');
		expect(formatCOP('')).toBe('$0');
		expect(formatCOP('abc')).toBe('$0');
		expect(formatCOP(Number.NaN)).toBe('$0');
	});

	it('soporta decimales cuando se pide explícitamente', () => {
		expect(norm(formatCOP(1500000.5, true))).toBe('$ 1.500.000,50');
		expect(norm(formatCOP('490000.00', true))).toBe('$ 490.000,00');
	});

	it('redondea fracciones al formatear sin decimales', () => {
		expect(norm(formatCOP(999.6))).toBe('$ 1.000');
	});
});

describe('formatDate', () => {
	it('formatea fechas ISO en formato corto es-CO', () => {
		// 15 de agosto de 2026
		expect(formatDate('2026-08-15')).toMatch(/ago/);
		expect(formatDate('2026-08-15')).toContain('2026');
	});

	it('acepta objetos Date', () => {
		const d = new Date(2026, 0, 5); // 5 ene 2026
		expect(formatDate(d)).toMatch(/ene/);
		expect(formatDate(d)).toContain('2026');
	});

	it('devuelve — para valores nulos o inválidos', () => {
		expect(formatDate(null)).toBe('—');
		expect(formatDate(undefined)).toBe('—');
		expect(formatDate('no-es-fecha')).toBe('—');
	});

	it('interpreta YYYY-MM-DD como fecha local (regresión zona horaria)', () => {
		// Regresión: `new Date('2026-08-10')` se trataba como medianoche UTC y
		// en zonas con offset negativo (Bogotá, UTC-5) mostraba "9 de ago" en
		// lugar de "10 de ago" (afectaba al panel del Agente SIMIT y a cualquier
		// fecha ISO de la app).
		const out = formatDate('2026-08-10');
		expect(out).toContain('10 de ago');
		expect(out).not.toContain('9 de ago');
		// Invariante independiente de la zona horaria del entorno: formatear la
		// fecha ISO debe dar el mismo resultado que formatear el Date local.
		expect(formatDate('2026-08-10')).toBe(formatDate(new Date(2026, 7, 10)));
	});
});

describe('formatDateTime', () => {
	it('incluye hora y minutos', () => {
		const out = formatDateTime('2026-08-15T14:30:00');
		expect(out).toContain('2026');
		// es-CO usa reloj 12 h: 14:30 → "02:30 p. m."
		expect(out).toMatch(/02:30/);
		expect(out).toMatch(/p\.\s?m\./);
	});

	it('devuelve — para nulos', () => {
		expect(formatDateTime(null)).toBe('—');
	});

	it('no retrocede el día con una fecha sin hora (regresión zona horaria)', () => {
		const out = formatDateTime('2026-08-10');
		expect(out).toContain('10 de ago');
		expect(out).not.toContain('9 de ago');
		expect(formatDateTime('2026-08-10')).toBe(formatDateTime(new Date(2026, 7, 10)));
	});
});

describe('formatContrato', () => {
	it('formatea secuencia por año con padding a 3 dígitos', () => {
		expect(formatContrato(2026, 1)).toBe('2026-001');
		expect(formatContrato(2026, 42)).toBe('2026-042');
		expect(formatContrato(2026, 1000)).toBe('2026-1000');
	});

	it('reinicia la secuencia cada año', () => {
		expect(formatContrato(2026, 7)).toBe('2026-007');
		expect(formatContrato(2027, 1)).toBe('2027-001');
	});

	it('devuelve — cuando falta año o secuencia', () => {
		expect(formatContrato(null, 1)).toBe('—');
		expect(formatContrato(2026, null)).toBe('—');
		expect(formatContrato(undefined, undefined)).toBe('—');
	});
});

describe('truncate', () => {
	it('trunca texto largo con elipsis', () => {
		expect(truncate('hola mundo', 5)).toBe('hola…');
	});

	it('no trunca texto corto', () => {
		expect(truncate('corto')).toBe('corto');
	});

	it('maneja valores vacíos', () => {
		expect(truncate('')).toBe('');
	});
});
