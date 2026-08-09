// src/lib/utils/calendario.test.ts — Tests de las utilidades puras de calendario
import { describe, it, expect } from 'vitest';
import {
	celdasDelMes,
	detectarSolapamientos,
	diasSemanaCorto,
	rangoCubreDia
} from './calendario';

describe('rangoCubreDia', () => {
	it('cubre los días del intervalo (inclusivo)', () => {
		expect(rangoCubreDia('2026-08-01', '2026-08-04', '2026-08-01')).toBe(true);
		expect(rangoCubreDia('2026-08-01', '2026-08-04', '2026-08-03')).toBe(true);
		expect(rangoCubreDia('2026-08-01', '2026-08-04', '2026-08-04')).toBe(true);
	});

	it('no cubre días fuera del intervalo', () => {
		expect(rangoCubreDia('2026-08-01', '2026-08-04', '2026-07-31')).toBe(false);
		expect(rangoCubreDia('2026-08-01', '2026-08-04', '2026-08-05')).toBe(false);
	});

	it('tolera fechas vacías o inválidas', () => {
		expect(rangoCubreDia('', '2026-08-04', '2026-08-01')).toBe(false);
		expect(rangoCubreDia('no-fecha', '2026-08-04', '2026-08-01')).toBe(false);
	});
});

describe('detectarSolapamientos', () => {
	it('detecta cruce de rangos de la misma placa', () => {
		const items = [
			{ id: 1, placa: 'ABC123', inicio: '2026-08-01', fin: '2026-08-05' },
			{ id: 2, placa: 'ABC123', inicio: '2026-08-04', fin: '2026-08-08' },
			{ id: 3, placa: 'XYZ987', inicio: '2026-08-01', fin: '2026-08-05' }
		];
		const solapes = detectarSolapamientos(items);
		expect(solapes).toEqual([{ a: 1, b: 2, placa: 'ABC123' }]);
	});

	it('no marca rangos contiguos ni de distinta placa', () => {
		const items = [
			{ id: 1, placa: 'ABC123', inicio: '2026-08-01', fin: '2026-08-04' },
			{ id: 2, placa: 'ABC123', inicio: '2026-08-05', fin: '2026-08-08' },
			{ id: 3, placa: 'XYZ987', inicio: '2026-08-03', fin: '2026-08-06' }
		];
		expect(detectarSolapamientos(items)).toEqual([]);
	});

	it('ignora items sin placa', () => {
		const items = [
			{ id: 1, placa: null, inicio: '2026-08-01', fin: '2026-08-05' },
			{ id: 2, placa: null, inicio: '2026-08-03', fin: '2026-08-08' }
		];
		expect(detectarSolapamientos(items)).toEqual([]);
	});
});

describe('celdasDelMes', () => {
	it('agosto 2026 empieza en lunes (celda inicial lunes 27-jul)', () => {
		// 1-ago-2026 es sábado → semana empieza el lunes 27-jul
		const semanas = celdasDelMes(2026, 7);
		expect(semanas).toHaveLength(6);
		expect(semanas[0][0].dia).toBe('2026-07-27');
		expect(semanas[0][0].enMes).toBe(false);
		// El día 1 de agosto cae en la primera semana, posición 5 (sábado)
		expect(semanas[0][5].dia).toBe('2026-08-01');
		expect(semanas[0][5].enMes).toBe(true);
	});

	it('cada semana tiene 7 celdas y la última celda es domingo', () => {
		const semanas = celdasDelMes(2026, 0); // enero 2026
		expect(semanas[0]).toHaveLength(7);
		const ultima = semanas[semanas.length - 1];
		const ultimoDia = new Date(ultima[ultima.length - 1].dia + 'T00:00:00');
		expect(ultimoDia.getDay()).toBe(0); // domingo
	});
});

describe('diasSemanaCorto', () => {
	it('empieza en lunes', () => {
		expect(diasSemanaCorto()[0]).toBe('lun');
		expect(diasSemanaCorto()[6]).toBe('dom');
	});
});
