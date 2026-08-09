// src/lib/components/AtajosModal.test.ts — Tests del modal de ayuda de atajos
// de teclado (F1 / Ctrl+/) y del detector esAtajoAyuda.
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import AtajosModal, { esAtajoAyuda, ATAJOS_APP } from './AtajosModal.svelte';

function renderModal(open: boolean) {
	return render(AtajosModal, { open, onClose: vi.fn() });
}

describe('AtajosModal', () => {
	it('muestra la lista de atajos cuando está abierto', () => {
		renderModal(true);

		expect(screen.getByRole('dialog')).toHaveTextContent('Atajos de teclado');
		// F1 y Ctrl+/ aparecen también en la línea de «consejo» al pie del modal
		expect(screen.getAllByText('F1').length).toBeGreaterThanOrEqual(2);
		expect(screen.getAllByText('Ctrl+/').length).toBeGreaterThanOrEqual(2);
		expect(screen.getByText('Ctrl+K')).toBeInTheDocument();
		expect(screen.getByText('Ctrl+Shift+C')).toBeInTheDocument();
		expect(screen.getByText('Esc')).toBeInTheDocument();
		expect(screen.getByText(/Copiar datos de un cliente o vehículo existente/)).toBeInTheDocument();
	});

	it('no renderiza nada cuando está cerrado', () => {
		renderModal(false);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(screen.queryByText('Atajos de teclado')).not.toBeInTheDocument();
	});

	it('expone la lista de atajos como fuente única', () => {
		expect(ATAJOS_APP.length).toBeGreaterThanOrEqual(3);
		for (const atajo of ATAJOS_APP) {
			expect(atajo.teclas.length).toBeGreaterThan(0);
			expect(atajo.descripcion.length).toBeGreaterThan(0);
		}
	});
});

describe('esAtajoAyuda', () => {
	const evento = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);

	it('detecta F1 solo', () => {
		expect(esAtajoAyuda(evento({ key: 'F1' }))).toBe(true);
	});

	it('detecta Ctrl+/ (sin otras modificadoras)', () => {
		expect(esAtajoAyuda(evento({ key: '/', ctrlKey: true }))).toBe(true);
	});

	it("no detecta '/' sin Ctrl", () => {
		expect(esAtajoAyuda(evento({ key: '/' }))).toBe(false);
	});

	it('no detecta Ctrl+Shift+C (es el atajo de copiar, no la ayuda)', () => {
		expect(esAtajoAyuda(evento({ key: 'c', ctrlKey: true, shiftKey: true }))).toBe(false);
	});

	it('no detecta F1 con modificadoras', () => {
		expect(esAtajoAyuda(evento({ key: 'F1', altKey: true }))).toBe(false);
		expect(esAtajoAyuda(evento({ key: 'F1', ctrlKey: true }))).toBe(false);
	});

	it('no detecta Ctrl+Alt+/ (combinación distinta)', () => {
		expect(esAtajoAyuda(evento({ key: '/', ctrlKey: true, altKey: true }))).toBe(false);
	});
});
