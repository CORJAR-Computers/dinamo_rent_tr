import { describe, expect, it } from 'vitest';
import { geografia, PAISES_BASE, DEPARTAMENTOS_COLOMBIA, CIUDADES_COLOMBIA } from './geografia';

describe('geografia', () => {
	it('el catálogo base de países es estable y no vacío', () => {
		expect(PAISES_BASE.length).toBeGreaterThan(10);
		expect(PAISES_BASE).toContain('Colombia');
	});

	it('el catálogo base de departamentos incluye los 32 + Bogotá D.C.', () => {
		expect(DEPARTAMENTOS_COLOMBIA.length).toBe(33);
		expect(DEPARTAMENTOS_COLOMBIA).toContain('Antioquia');
		expect(DEPARTAMENTOS_COLOMBIA).toContain('Bogotá D.C.');
	});

	it('el catálogo base de ciudades incluye las principales', () => {
		expect(CIUDADES_COLOMBIA).toContain('Bogotá');
		expect(CIUDADES_COLOMBIA).toContain('Barranquilla');
		expect(CIUDADES_COLOMBIA).toContain('Santa Marta');
	});

	it('sin valores usados devuelve el catálogo base intacto', () => {
		expect(geografia.paises()).toEqual(PAISES_BASE);
		expect(geografia.departamentos()).toEqual(DEPARTAMENTOS_COLOMBIA);
	});

	it('añade valores usados nuevos al final, ordenados alfabéticamente', () => {
		const paises = geografia.paises(['Zimbabue', 'Colombia']);
		expect(paises).toContain('Colombia');
		expect(paises).toContain('Zimbabue');
		// Los del catálogo base mantienen su orden; los nuevos van al final
		expect(paises.slice(PAISES_BASE.length)).toEqual(['Zimbabue']);
	});

	it('ignora valores vacíos y duplicados', () => {
		const ciudades = geografia.ciudades(['', '   ', 'Bogotá', 'Bogotá']);
		const unicas = new Set(ciudades);
		expect(unicas.size).toBe(ciudades.length);
		expect(ciudades.filter((c) => c === 'Bogotá').length).toBe(1);
		expect(ciudades.every((c) => c.trim().length > 0)).toBe(true);
	});

	it('ordena los extras con locale es (acentos incluidos)', () => {
		const departamentos = geografia.departamentos(['Zulia', 'Ávila']);
		// 'Ávila' con locale es va antes que 'Amazonas' (base) y antes que 'Zulia'
		const posAvila = departamentos.indexOf('Ávila');
		const posZulia = departamentos.indexOf('Zulia');
		expect(posAvila).toBeGreaterThan(0);
		expect(posZulia).toBeGreaterThan(posAvila);
	});
});
