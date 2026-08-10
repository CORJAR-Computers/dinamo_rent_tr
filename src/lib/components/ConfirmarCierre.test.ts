// src/lib/components/ConfirmarCierre.test.ts — Tests del diálogo de
// confirmación de cierre (botón X de la ventana): el backend emite
// 'app-close-requested', el modal se abre, «No» cancela y «Sí» invoca el
// comando confirmar_cierre.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import ConfirmarCierre from './ConfirmarCierre.svelte';
import { tauri } from '../../test/tauri';
import { listen } from '@tauri-apps/api/event';

// Mock del módulo de eventos de Tauri: capturamos el handler para poder
// simular la llegada del evento 'app-close-requested' en los tests.
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn()
}));

const listenMock = vi.mocked(listen);
let handler: ((e: unknown) => void) | null = null;
const confirmarSpy = vi.fn();
const frontendListaSpy = vi.fn();

beforeEach(() => {
	handler = null;
	listenMock.mockReset();
	listenMock.mockImplementation(((_event: string, cb: (e: unknown) => void) => {
		handler = cb;
		return Promise.resolve(() => {
			handler = null;
		});
	}) as never);
	confirmarSpy.mockReset();
	frontendListaSpy.mockReset();
	tauri.register('confirmar_cierre', confirmarSpy);
	tauri.register('app_frontend_lista', frontendListaSpy);
});

/** Dispara el evento de cierre del backend tal y como llega desde Tauri. */
async function emitirCierre() {
	await waitFor(() => expect(listenMock).toHaveBeenCalled());
	handler?.({ event: 'app-close-requested', id: 0, payload: undefined });
}

describe('ConfirmarCierre', () => {
	it('no muestra el diálogo inicialmente', () => {
		render(ConfirmarCierre);

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(listenMock).toHaveBeenCalledWith('app-close-requested', expect.any(Function));
	});

	it('avisa al backend que el frontend está listo al montar', async () => {
		render(ConfirmarCierre);

		await waitFor(() => expect(frontendListaSpy).toHaveBeenCalledTimes(1));
	});

	it('muestra el diálogo al llegar el evento de cierre del backend', async () => {
		render(ConfirmarCierre);

		await emitirCierre();

		expect(screen.getByRole('dialog')).toHaveTextContent('Cerrar aplicación');
		expect(
			screen.getByText('¿Está seguro de cerrar la aplicación?')
		).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Sí' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'No' })).toBeInTheDocument();
	});

	it('enfoca «No» al abrir (acción segura)', async () => {
		render(ConfirmarCierre);

		await emitirCierre();

		expect(screen.getByRole('button', { name: 'No' })).toHaveFocus();
	});

	it('«No» cierra el diálogo sin invocar confirmar_cierre', async () => {
		render(ConfirmarCierre);

		await emitirCierre();
		fireEvent.click(screen.getByRole('button', { name: 'No' }));

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(confirmarSpy).not.toHaveBeenCalled();
	});

	it('«Sí» invoca confirmar_cierre y cierra el diálogo', async () => {
		render(ConfirmarCierre);

		await emitirCierre();
		fireEvent.click(screen.getByRole('button', { name: 'Sí' }));

		await waitFor(() => expect(confirmarSpy).toHaveBeenCalledTimes(1));
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});

	it('se cierra con Esc (equivale a «No»)', async () => {
		render(ConfirmarCierre);

		await emitirCierre();
		expect(screen.getByRole('dialog')).toBeInTheDocument();

		fireEvent.keyDown(document, { key: 'Escape' });

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(confirmarSpy).not.toHaveBeenCalled();
	});

	it('no reintenta abrir si el evento llega con el diálogo ya abierto', async () => {
		render(ConfirmarCierre);

		await emitirCierre();
		await emitirCierre();

		// Solo un diálogo en el DOM
		expect(screen.getAllByRole('dialog')).toHaveLength(1);
	});
});
