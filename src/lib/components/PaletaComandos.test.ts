// src/lib/components/PaletaComandos.test.ts — Tests de la command palette
// (Ctrl+K): filtrado, navegación por teclado/mouse, roles y helpers.
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { goto } from '$app/navigation';
import PaletaComandos, {
	esAtajoPaleta,
	normalizarTexto,
	type SeccionMenu
} from './PaletaComandos.svelte';

const menu: SeccionMenu[] = [
	{ section: 'PRINCIPAL', items: [{ label: 'Dashboard', href: '/dashboard', icon: 'dashboard' }] },
	{
		section: 'OPERACIÓN',
		items: [
			{ label: 'Calendario', href: '/calendario', icon: 'calendar' },
			{ label: 'Rentas', href: '/rentas', icon: 'rentas' },
			{ label: 'Reservas', href: '/reservas', icon: 'reservas' },
			{ label: 'Clientes', href: '/clientes', icon: 'clientes' },
			{ label: 'Autos', href: '/autos', icon: 'autos' }
		]
	},
	{
		section: 'ADMINISTRACIÓN',
		items: [
			{ label: 'Usuarios', href: '/usuarios', icon: 'usuarios', adminOnly: true },
			{ label: 'Auditoría', href: '/auditoria', icon: 'auditoria', adminOnly: true },
			{ label: 'Alertas', href: '/alertas', icon: 'alertas' }
		]
	},
	{
		section: 'FINANZAS',
		items: [
			{ label: 'Informes', href: '/informes', icon: 'informes' },
			{ label: 'Gastos', href: '/gastos', icon: 'gastos' }
		]
	}
];

function renderPaleta(props: { open: boolean; rol?: string | null; actualPath?: string }) {
	return render(PaletaComandos, { menu, onClose: vi.fn(), ...props });
}

/** Escribe en el campo de búsqueda (bind:value escucha 'input'). */
function escribir(termino: string) {
	fireEvent.input(screen.getByRole('combobox'), { target: { value: termino } });
}

describe('PaletaComandos', () => {
	it('muestra los resultados cuando está abierto', () => {
		renderPaleta({ open: true });

		expect(screen.getByRole('dialog')).toHaveTextContent('Ir a…');
		expect(screen.getByRole('combobox', { name: 'Buscar página' })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Rentas/ })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Dashboard/ })).toBeInTheDocument();
	});

	it('no renderiza nada cuando está cerrado', () => {
		renderPaleta({ open: false });

		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
	});

	it('enfoca el campo de búsqueda al abrir', async () => {
		renderPaleta({ open: true });

		await waitFor(() => expect(screen.getByRole('combobox')).toHaveFocus());
	});

	it('filtra por nombre sin distinguir acentos', () => {
		// Auditoría es adminOnly: se necesita el rol para que aparezca en la lista
		renderPaleta({ open: true, rol: 'Administrador' });

		escribir('auditoria'); // sin tilde

		expect(screen.getByRole('option', { name: /Auditoría/ })).toBeInTheDocument();
		expect(screen.queryByRole('option', { name: /Rentas/ })).not.toBeInTheDocument();
		expect(screen.queryByRole('option', { name: /Gastos/ })).not.toBeInTheDocument();
	});

	it('filtra también por nombre de sección', () => {
		renderPaleta({ open: true });

		escribir('finanzas');

		expect(screen.getByRole('option', { name: /Gastos/ })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Informes/ })).toBeInTheDocument();
		expect(screen.queryByRole('option', { name: /Rentas/ })).not.toBeInTheDocument();
	});

	it('oculta los ítems adminOnly para roles sin permisos y los muestra para Administrador', () => {
		renderPaleta({ open: true, rol: 'Operador' });

		expect(screen.queryByRole('option', { name: /Usuarios/ })).not.toBeInTheDocument();
		expect(screen.queryByRole('option', { name: /Auditoría/ })).not.toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Alertas/ })).toBeInTheDocument();

		renderPaleta({ open: true, rol: 'Administrador' });
		expect(screen.getByRole('option', { name: /Usuarios/ })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Auditoría/ })).toBeInTheDocument();
	});

	it('navega con ↑↓ y Enter', () => {
		// Menú dedicado de 2 ítems para que el orden del filtro sea determinista
		const menuCorto: SeccionMenu[] = [
			{
				section: 'OPERACIÓN',
				items: [
					{ label: 'Rentas', href: '/rentas', icon: 'rentas' },
					{ label: 'Reservas', href: '/reservas', icon: 'reservas' }
				]
			}
		];
		const onClose = vi.fn();
		render(PaletaComandos, { open: true, onClose, menu: menuCorto });
		const input = screen.getByRole('combobox');

		escribir('r');
		// Resultados: Rentas (0), Reservas (1) — 'r' matchea ambos
		fireEvent.keyDown(input, { key: 'ArrowDown' });
		fireEvent.keyDown(input, { key: 'Enter' });

		expect(goto).toHaveBeenCalledWith('/reservas');
		expect(onClose).toHaveBeenCalledTimes(1);
	});

	it('navega al hacer clic en un resultado', () => {
		renderPaleta({ open: true });

		fireEvent.click(screen.getByRole('option', { name: /Gastos/ }));

		expect(goto).toHaveBeenCalledWith('/gastos');
	});

	it('se cierra con Esc', () => {
		const onClose = vi.fn();
		render(PaletaComandos, { open: true, onClose, menu });

		// El listener de Esc vive en Modal (document)
		fireEvent.keyDown(document, { key: 'Escape' });

		expect(onClose).toHaveBeenCalledTimes(1);
	});

	it('muestra estado vacío cuando no hay coincidencias', () => {
		renderPaleta({ open: true });

		escribir('zzzz');

		expect(screen.getByText(/Sin resultados para «zzzz»/)).toBeInTheDocument();
		expect(screen.queryByRole('option')).not.toBeInTheDocument();
	});

	it('marca la página actual', () => {
		renderPaleta({ open: true, actualPath: '/rentas' });

		expect(screen.getByRole('option', { name: /Rentas/ })).toHaveTextContent('Actual');
		expect(screen.getByRole('option', { name: /Reservas/ })).not.toHaveTextContent('Actual');
	});

	it('limpia la búsqueda al reabrir', async () => {
		const { rerender } = renderPaleta({ open: true });

		escribir('ren');
		expect((screen.getByRole('combobox') as HTMLInputElement).value).toBe('ren');

		rerender({ open: false });
		rerender({ open: true });

		await waitFor(() => expect((screen.getByRole('combobox') as HTMLInputElement).value).toBe(''));
	});
});

describe('esAtajoPaleta', () => {
	const evento = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);

	it('detecta Ctrl+K', () => {
		expect(esAtajoPaleta(evento({ key: 'k', ctrlKey: true }))).toBe(true);
	});

	it('detecta Cmd+K (macOS)', () => {
		expect(esAtajoPaleta(evento({ key: 'k', metaKey: true }))).toBe(true);
	});

	it('no detecta "k" sin modificadoras', () => {
		expect(esAtajoPaleta(evento({ key: 'k' }))).toBe(false);
	});

	it('no detecta Ctrl+Shift+K ni Ctrl+Alt+K', () => {
		expect(esAtajoPaleta(evento({ key: 'k', ctrlKey: true, shiftKey: true }))).toBe(false);
		expect(esAtajoPaleta(evento({ key: 'k', ctrlKey: true, altKey: true }))).toBe(false);
	});

	it('no detecta Ctrl+Shift+C (es el atajo de copiar, no la paleta)', () => {
		expect(esAtajoPaleta(evento({ key: 'c', ctrlKey: true, shiftKey: true }))).toBe(false);
	});
});

describe('normalizarTexto', () => {
	it('elimina acentos y pasa a minúsculas', () => {
		expect(normalizarTexto('Auditoría')).toBe('auditoria');
		expect(normalizarTexto('RENTAS')).toBe('rentas');
		expect(normalizarTexto('Administración')).toBe('administracion');
	});
});
