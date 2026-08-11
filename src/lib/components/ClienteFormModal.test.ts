// src/lib/components/ClienteFormModal.test.ts — Tests del panel «Copiar datos
// de un cliente existente» (buscar/duplicar) montando el componente directo.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Cliente, ClienteConPii, BusinessLists } from '$lib/api';
import ClienteFormModal from './ClienteFormModal.svelte';

function cliente(overrides: Partial<Cliente> = {}): Cliente {
	return {
		id: 1,
		tipoDoc: 'Cédula',
		noDoc: '1036672369',
		nombres: 'Ana',
		apellidos: 'Pérez',
		nombreCompleto: 'Ana Pérez',
		celular: '3101234567',
		celular2: null,
		email: 'ana@correo.com',
		ciudad: 'Barranquilla',
		estadoRegion: null,
		pais: 'Colombia',
		nacionalidad: 'Colombiana',
		dirResidencia: null,
		dirTemporal: null,
		hotel: null,
		habitacion: null,
		noLicencia: null,
		tipoLicencia: null,
		vencimientoLicencia: null,
		estado: 'Activo',
		createdAt: null,
		...overrides
	};
}

function conPii(c: Cliente, piiOculto = false): ClienteConPii {
	return { cliente: c, piiOculto };
}

const LISTS: BusinessLists = {
	tiposAuto: [],
	tiposTransmision: [],
	tiposCombustible: [],
	estadosAuto: [],
	tiposAdquisicion: [],
	tiposDoc: ['Cédula', 'Pasaporte', 'Cédula Extranjería', 'NIT', 'Licencia USA'],
	estadosCliente: ['Activo', 'Inactivo', 'Lista Negra', 'VIP'],
	estadosReserva: [],
	tiposGasto: [],
	nivelTanque: [],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesConEliminar: [],
	rolesDisponibles: []
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function renderModal() {
	return render(ClienteFormModal, {
		open: true,
		editando: null,
		lists: LISTS,
		clientes: [],
		onClose: vi.fn(),
		onGuardado: vi.fn()
	});
}

async function abrirPanel() {
	await fireEvent.click(screen.getByRole('button', { name: /Copiar datos de un cliente existente/ }));
	return screen.getByPlaceholderText('Buscar por nombre, documento o celular…');
}

beforeEach(() => {
	session.clear();
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: 'admin',
		nombre: 'Administrador',
		rol: 'Administrador',
		debeCambiarPassword: false
	});
});

describe('panel copiar cliente', () => {
	it('busca con debounce y exige al menos 2 caracteres', async () => {
		const listar = vi.fn((_args: { busqueda: string | null }) => [
			conPii(cliente({ id: 2, nombreCompleto: 'Luis Gómez', noDoc: '72145678' }))
		]);
		tauri.register('listar_clientes', listar);

		renderModal();
		const input = await abrirPanel();

		// 1 carácter → no dispara la búsqueda
		await fireEvent.input(input, { target: { value: 'a' } });
		await sleep(350); // supera el debounce de 300 ms
		expect(listar).not.toHaveBeenCalled();

		// Escritura rápida → una sola llamada (debounce) con el término final
		await fireEvent.input(input, { target: { value: 'lu' } });
		await fireEvent.input(input, { target: { value: 'lui' } });
		await fireEvent.input(input, { target: { value: 'luis' } });
		await waitFor(() => expect(listar).toHaveBeenCalledTimes(1), { timeout: 2000 });
		const args = listar.mock.calls[0][0] as { busqueda: string | null };
		expect(args.busqueda).toBe('luis');

		// Los resultados se muestran
		expect(await screen.findByText('Luis Gómez')).toBeInTheDocument();
	});

	it('duplica los datos del cliente y limpia el documento (es único)', async () => {
		tauri.register('listar_clientes', () => [
			conPii(
				cliente({
					id: 2,
					nombres: 'Luis',
					apellidos: 'Gómez',
					nombreCompleto: 'Luis Gómez',
					noDoc: '72145678',
					celular: '3001112222',
					email: 'luis@correo.com'
				})
			)
		]);

		renderModal();
		const input = await abrirPanel();
		await fireEvent.input(input, { target: { value: 'luis' } });

		await fireEvent.click(await screen.findByText('Luis Gómez'));

		// Datos copiados al formulario
		const nombres = screen.getByPlaceholderText('Nombres del cliente') as HTMLInputElement;
		const apellidos = screen.getByPlaceholderText('Apellidos del cliente') as HTMLInputElement;
		const celular = screen.getByPlaceholderText('Ej: 3101234567') as HTMLInputElement;
		const email = screen.getByPlaceholderText('cliente@correo.com') as HTMLInputElement;
		const noDoc = screen.getByPlaceholderText('Ej: 1036672369') as HTMLInputElement;
		expect(nombres.value).toBe('Luis');
		expect(apellidos.value).toBe('Gómez');
		expect(celular.value).toBe('3001112222');
		expect(email.value).toBe('luis@correo.com');
		// El documento debe quedar vacío (único en la BD)
		expect(noDoc.value).toBe('');

		// Aviso de datos copiados + foco en el campo de documento
		expect(screen.getByText(/Datos copiados de/)).toBeInTheDocument();
		await waitFor(() => expect(noDoc).toHaveFocus());
	});

	it('formatea el subtitulo de los resultados con y sin documento (incluye ciudad)', async () => {
		tauri.register('listar_clientes', () => [
			conPii(
				cliente({
					id: 2,
					nombres: 'Luis',
					apellidos: 'Gómez',
					nombreCompleto: 'Luis Gómez',
					noDoc: '72145678',
					ciudad: 'Medellín'
				})
			),
			conPii(
				cliente({
					id: 3,
					nombres: 'Sara',
					apellidos: 'Díaz',
					nombreCompleto: 'Sara Díaz',
					noDoc: null,
					tipoDoc: 'Pasaporte',
					ciudad: 'Cartagena'
				})
			)
		]);

		renderModal();
		const input = await abrirPanel();
		await fireEvent.input(input, { target: { value: 'lu' } });

		await screen.findByText('Luis Gómez');
		// Con documento: «Tipo: NoDoc · Ciudad»
		expect(screen.getByText('Cédula: 72145678 · Medellín')).toBeInTheDocument();
		// Sin documento: «Tipo · Ciudad» (la ciudad nunca se pierde)
		expect(screen.getByText('Pasaporte · Cartagena')).toBeInTheDocument();
	});

	it('bloquea la copia de clientes con PII oculta (legacy)', async () => {
		tauri.register('listar_clientes', () => [
			conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }), true),
			conPii(
				cliente({
					id: 2,
					nombres: 'Luis',
					apellidos: 'Gómez',
					nombreCompleto: 'Luis Gómez',
					noDoc: '72145678'
				}),
				false
			)
		]);

		renderModal();
		const input = await abrirPanel();
		await fireEvent.input(input, { target: { value: 'lu' } });

		// Ambos aparecen en los resultados
		expect(await screen.findByText('Ana Pérez')).toBeInTheDocument();
		expect(screen.getByText('Luis Gómez')).toBeInTheDocument();

		// El cliente con PII oculta está deshabilitado (no se puede duplicar)
		const botonAna = screen.getByTitle('Tiene datos cifrados con clave antigua: no se pueden copiar.');
		const botonLuis = screen.getByTitle('Copiar datos de Luis Gómez');
		expect(botonAna).toBeDisabled();
		expect(botonLuis).not.toBeDisabled();

		// El cliente sin restricción sí rellena el formulario
		await fireEvent.click(botonLuis);
		const nombres = screen.getByPlaceholderText('Nombres del cliente') as HTMLInputElement;
		await waitFor(() => expect(nombres.value).toBe('Luis'));
	});

	it('descarta respuestas obsoletas de la búsqueda (race token)', async () => {
		let llamada = 0;
		let resolverA!: (v: ClienteConPii[]) => void;
		const promA = new Promise<ClienteConPii[]>((res) => (resolverA = res));
		tauri.register('listar_clientes', () => {
			llamada++;
			if (llamada === 1) return promA; // primera búsqueda queda en vuelo
			return Promise.resolve([conPii(cliente({ id: 2, nombreCompleto: 'Luis Gómez' }))]);
		});

		renderModal();
		const input = await abrirPanel();

		// Primera búsqueda: en vuelo (la promesa no se resuelve aún)
		await fireEvent.input(input, { target: { value: 'ja' } });
		await waitFor(() => expect(llamada).toBe(1), { timeout: 2000 });

		// Segunda búsqueda: resuelve de inmediato con Luis
		await fireEvent.input(input, { target: { value: 'javi' } });
		await waitFor(() => expect(llamada).toBe(2), { timeout: 2000 });
		expect(await screen.findByText('Luis Gómez')).toBeInTheDocument();

		// La respuesta vieja (Ana) llega tarde → debe ignorarse
		resolverA([conPii(cliente({ id: 1, nombreCompleto: 'Ana Pérez' }))]);
		await sleep(50);
		expect(screen.queryByText('Ana Pérez')).not.toBeInTheDocument();
		expect(screen.getByText('Luis Gómez')).toBeInTheDocument();
	});
});
