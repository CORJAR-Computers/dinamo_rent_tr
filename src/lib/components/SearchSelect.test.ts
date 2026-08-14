// src/lib/components/SearchSelect.test.ts — Tests del combobox con búsqueda:
// filtrado por escritura (nombre y número de documento), selección con teclado
// y con clic, y sincronización del valor controlado.
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import SearchSelect, { type SearchSelectOpcion } from './SearchSelect.svelte';

const opciones: SearchSelectOpcion[] = [
	{ value: '1', label: 'MARIA FERNANDA PEREZ', sub: 'CC 1045678912' },
	{ value: '2', label: 'CARLOS ANDRES GOMEZ', sub: 'CC 1002345678' },
	{ value: '3', label: 'JOSE ANTONIO RUIZ', sub: 'TI 1023456789' },
	{ value: '4', label: 'Ana María López', sub: 'CC 987654321' }
];

async function montar(value = '', onchange = vi.fn()) {
	const utils = render(SearchSelect, {
		props: {
			label: 'Cliente',
			value,
			opciones,
			onchange,
			placeholder: 'Buscar…',
			vacioLabel: '— Sin cliente —'
		}
	});
	const input = screen.getByRole('combobox');
	return { utils, input, onchange };
}

describe('SearchSelect', () => {
	it('abre la lista con todas las opciones al enfocar', async () => {
		const { input } = await montar();
		await fireEvent.focus(input);
		expect(screen.getAllByRole('option').length).toBe(opciones.length + 1); // + la opción vacía
		expect(screen.getByText('MARIA FERNANDA PEREZ')).toBeTruthy();
	});

	it('filtra por nombre ignorando mayúsculas y tildes', async () => {
		const { input } = await montar();
		await fireEvent.focus(input);
		await fireEvent.input(input, { target: { value: 'ana maria' } });
		const visibles = screen.getAllByRole('option').filter((el) => el.textContent?.includes('López'));
		expect(visibles.length).toBe(1);
		expect(screen.getByText('Ana María López')).toBeTruthy();
		expect(screen.queryByText('MARIA FERNANDA PEREZ')).toBeNull();
	});

	it('filtra por número de documento (sub)', async () => {
		const { input } = await montar();
		await fireEvent.focus(input);
		await fireEvent.input(input, { target: { value: '1023456789' } });
		expect(screen.getByText('JOSE ANTONIO RUIZ')).toBeTruthy();
		expect(screen.queryByText('CARLOS ANDRES GOMEZ')).toBeNull();
	});

	it('selecciona con clic y notifica el valor', async () => {
		const { input, onchange } = await montar();
		await fireEvent.focus(input);
		await fireEvent.click(screen.getByText('CARLOS ANDRES GOMEZ'));
		expect(onchange).toHaveBeenCalledWith('2');
	});

	it('selecciona con teclado (flecha abajo + Enter)', async () => {
		const { input, onchange } = await montar();
		await fireEvent.focus(input);
		await fireEvent.keyDown(input, { key: 'ArrowDown' });
		await fireEvent.keyDown(input, { key: 'Enter' });
		expect(onchange).toHaveBeenCalledWith('1');
	});

	it('muestra el label del valor seleccionado y permite deseleccionar', async () => {
		const { input, onchange } = await montar('2');
		expect((input as HTMLInputElement).value).toBe('CARLOS ANDRES GOMEZ');
		await fireEvent.focus(input);
		await fireEvent.click(screen.getByText('— Sin cliente —'));
		expect(onchange).toHaveBeenCalledWith('');
	});

	it('muestra «Sin coincidencias» cuando nada matchea', async () => {
		const { input } = await montar();
		await fireEvent.focus(input);
		await fireEvent.input(input, { target: { value: 'zzzz' } });
		expect(screen.getByText(/Sin coincidencias/)).toBeTruthy();
	});
});
