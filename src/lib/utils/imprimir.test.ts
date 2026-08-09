// imprimir.test.ts — Tests del flujo de impresión (imprimirDocumento).
// Verifica que al imprimir se renombre document.title con el nombre del
// documento (para el encabezado propio del diálogo de impresión) y que se
// restaure al terminar, incluida la limpieza del clon.
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { imprimirDocumento } from './imprimir';

function montar(html: string): void {
	document.body.innerHTML = html;
}

describe('imprimirDocumento', () => {
	beforeEach(() => {
		document.title = 'Rentas — Dinamo Rent ERP';
		vi.useFakeTimers();
		// jsdom no implementa window.print; lo reemplazamos por un stub
		Object.defineProperty(window, 'print', { value: vi.fn(), configurable: true, writable: true });
	});

	afterEach(() => {
		document.body.innerHTML = '';
		vi.useRealTimers();
	});

	it('renombra el título al imprimir un contrato y lo restaura al terminar', async () => {
		montar('<div class="print-area contrato-carta">Contrato de prueba</div>');

		imprimirDocumento();

		// El título se cambia en el .then() del clon listo (microtarea)
		await Promise.resolve();
		await Promise.resolve();
		expect(document.title).toBe('Contrato de renta');

		// La limpieza ocurre vía setTimeout(1000) (fallback de afterprint)
		await vi.advanceTimersByTimeAsync(1200);
		expect(document.title).toBe('Rentas — Dinamo Rent ERP');
		expect(document.querySelector('#print-clone')).toBeNull();
		expect(document.body.classList.contains('printing')).toBe(false);
	});

	it('renombra el título al imprimir una orden de renta y lo restaura', async () => {
		montar('<div class="print-area orden-carta">Orden de prueba</div>');

		imprimirDocumento();

		await Promise.resolve();
		await Promise.resolve();
		expect(document.title).toBe('Orden de renta');

		await vi.advanceTimersByTimeAsync(1200);
		expect(document.title).toBe('Rentas — Dinamo Rent ERP');
	});

	it('no cambia el título para documentos sin marcador conocido', async () => {
		montar('<div class="print-area">Otro documento</div>');

		imprimirDocumento();

		await Promise.resolve();
		await Promise.resolve();
		expect(document.title).toBe('Rentas — Dinamo Rent ERP');

		await vi.advanceTimersByTimeAsync(1200);
		expect(document.title).toBe('Rentas — Dinamo Rent ERP');
	});
});
