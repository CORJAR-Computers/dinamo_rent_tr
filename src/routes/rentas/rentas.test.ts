// src/routes/rentas/rentas.test.ts — Tests de la página de Rentas
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/svelte';
import { tauri } from '../../test/tauri';
import { session } from '$lib/stores/session.svelte';
import type { Renta, RentaDatos, RentaCierreDatos, PagoDatos, InspeccionDatos, Auto, BusinessLists } from '$lib/api';
import RentasPage from './+page.svelte';

function renta(overrides: Partial<Renta> = {}): Renta {
	return {
		id: 1,
		noContrato: 42,
		anioContrato: 2026,
		placa: 'ABC123',
		idCliente: null,
		nombreCliente: 'Cliente de Prueba',
		noLicencia: null,
		nacionalidad: 'Colombiana',
		fechaRecogida: '2026-08-01',
		horaRecogida: '09:00',
		ubicacionRecogida: null,
		fechaRetorno: '2026-08-04',
		horaRetorno: '18:00',
		ubicacionRetorno: null,
		diasCalculados: 3,
		horasExtras: 0,
		valorDia: '150000.00',
		valorHoraExtra: '10000.00',
		valorDiaExtra: '0.00',
		costoLavado: '0.00',
		costoSilla: '0.00',
		costoRetorno: '0.00',
		costoDomicilio: '0.00',
		costoCables: '0.00',
		costoInversor: '0.00',
		descuento: '0.00',
		subtotal: '450000.00',
		impuestos: '85500.00',
		total: '535500.00',
		abono: '0.00',
		saldoPendiente: '535500.00',
		estado: 'Activo',
		observaciones: null,
		fechaDevolucionReal: null,
		horaDevolucionReal: null,
		kmFinal: null,
		tanqueFinal: null,
		kmSalida: '42000',
		tanqueSalida: 'Lleno',
		idReserva: null,
		createdAt: null,
		vehiculo: 'Toyota Corolla',
		pagos: [],
		inspecciones: [],
		...overrides
	};
}

function auto(placa: string, marca = 'Toyota', modelo = 'Corolla'): Auto {
	return {
		placa,
		marca,
		modelo,
		version: null,
		color: null,
		tipo: 'Automóvil',
		cilindraje: null,
		transmision: null,
		combustible: null,
		noMotor: null,
		noChasis: null,
		propietario: null,
		estado: 'Disponible',
		costoFijoMensual: '1500000',
		kilometraje: 42000,
		ubicacion: null,
		tipoAdquisicion: null,
		proximoAceite: null,
		proximoFrenos: null,
		vencimientoSoat: null,
		vencimientoTecnico: null,
		vencimientoExtintor: null,
		vencimientoBateria: null,
		observaciones: null,
		fechaIngreso: '2026-01-10',
		createdAt: null
	};
}

const LISTS: BusinessLists = {
	tiposAuto: [],
	tiposTransmision: [],
	tiposCombustible: [],
	estadosAuto: [],
	tiposAdquisicion: [],
	tiposDoc: [],
	estadosCliente: [],
	estadosReserva: [],
	tiposGasto: [],
	nivelTanque: ['Lleno', '3/4', '1/2', '1/4', 'Vacío'],
	tiposMantenimiento: [],
	rolesConInformes: [],
	rolesConUsuarios: [],
	rolesDisponibles: []
};

function setSesion() {
	session.setSession({
		success: true,
		sessionId: 'tok-test',
		username: 'admin',
		nombre: 'Administrador',
		rol: 'Administrador',
		debeCambiarPassword: false
	});
}

beforeEach(() => {
	session.clear();
	setSesion();
	tauri.register('get_business_lists', () => LISTS);
	tauri.register('listar_clientes', () => []);
	tauri.register('listar_autos', () => [auto('ABC123'), auto('XYZ987', 'Mazda', 'CX-5')]);
});

describe('página de Rentas', () => {
	it('lista las rentas con totales y estado', async () => {
		tauri.register('listar_rentas', () => [
			renta(),
			renta({ id: 2, noContrato: 43, placa: 'XYZ987', nombreCliente: 'Otro Cliente', estado: 'Cerrada', total: '714000.00' })
		]);

		render(RentasPage);

		expect(await screen.findByText('Cliente de Prueba')).toBeInTheDocument();
		expect(screen.getByText('Otro Cliente')).toBeInTheDocument();
		// Número de contrato por año visible en el listado (2026-042, 2026-043)
		expect(screen.getByText('2026-042')).toBeInTheDocument();
		expect(screen.getByText('2026-043')).toBeInTheDocument();
		// Totales en formato COP (Intl puede insertar espacio entre $ y el número;
		// total y saldo pueden coincidir, por eso getAllByText)
		expect(screen.getAllByText((c) => c.includes('535.500')).length).toBeGreaterThan(0);
		expect(screen.getAllByText((c) => c.includes('714.000')).length).toBeGreaterThan(0);
		// Estados (también aparecen como opciones del filtro)
		expect(screen.getAllByText('Activo').length).toBeGreaterThan(0);
		expect(screen.getAllByText('Cerrada').length).toBeGreaterThan(0);
		expect(screen.getByText(/2 rentas/)).toBeInTheDocument();
	});

	it('muestra el estado vacío cuando no hay rentas', async () => {
		tauri.register('listar_rentas', () => []);

		render(RentasPage);

		expect(await screen.findByText('No hay rentas')).toBeInTheDocument();
		expect(screen.getByText(/0 rentas/)).toBeInTheDocument();
	});

	it('crea una renta desde el modal', async () => {
		tauri.register('listar_rentas', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: RentaDatos }) => renta({ id: 9 }));
		tauri.register('crear_renta', crear);

		render(RentasPage);
		await screen.findByText('No hay rentas');

		await fireEvent.click(screen.getByRole('button', { name: 'Nueva Renta' }));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toBeInTheDocument();

		// Cliente (texto libre), placa y km de salida
		await fireEvent.input(screen.getByPlaceholderText('Nombre para la renta'), {
			target: { value: 'Cliente Nuevo' }
		});
		await fireEvent.change(within(dialogo).getByDisplayValue('— Seleccionar vehículo —'), {
			target: { value: 'ABC123' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Ej: 42000'), {
			target: { value: '42100' }
		});
		// Tarifa y abono
		await fireEvent.input(screen.getByPlaceholderText('Ej: 150000'), {
			target: { value: '150000' }
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Crear renta' }));

		await waitFor(() => expect(crear).toHaveBeenCalledTimes(1));
		const args = crear.mock.calls[0][0] as { sessionId: string; datos: RentaDatos };
		expect(args.datos.nombreCliente).toBe('Cliente Nuevo');
		expect(args.datos.placa).toBe('ABC123');
		expect(args.datos.kmSalida).toBe('42100');
		await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
	});

	it('valida los campos obligatorios antes de guardar', async () => {
		tauri.register('listar_rentas', () => []);
		const crear = vi.fn((_args: { sessionId: string; datos: RentaDatos }) => renta());
		tauri.register('crear_renta', crear);

		render(RentasPage);
		await screen.findByText('No hay rentas');

		await fireEvent.click(screen.getByRole('button', { name: 'Nueva Renta' }));
		await screen.findByRole('dialog');
		await fireEvent.click(screen.getByRole('button', { name: 'Crear renta' }));

		await waitFor(() => {
			expect(screen.getByRole('alert')).toHaveTextContent('El nombre del cliente es obligatorio.');
		});
		expect(crear).not.toHaveBeenCalled();
	});

	it('cierra una renta registrando la devolución real', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 5 })]);
		const cerrar = vi.fn((_args: { sessionId: string; id: number; datos: RentaCierreDatos }) =>
			renta({ id: 5, estado: 'Cerrada', kmFinal: '43100' })
		);
		tauri.register('cerrar_renta', cerrar);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		await fireEvent.click(screen.getByTitle('Cerrar renta (devolución)'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Cerrar renta #5');

		await fireEvent.input(screen.getByPlaceholderText('Km al devolver'), {
			target: { value: '43100' }
		});

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Cerrar renta' }));

		await waitFor(() => expect(cerrar).toHaveBeenCalledTimes(1));
		const args = cerrar.mock.calls[0][0] as { sessionId: string; id: number; datos: RentaCierreDatos };
		expect(args.id).toBe(5);
		expect(args.datos.kmFinal).toBe('43100');
		expect(args.datos.fechaDevolucionReal).toBeTruthy();
	});

	it('registra un pago contra una renta activa', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 5, saldoPendiente: '535500.00' })]);
		const pagar = vi.fn((_args: { sessionId: string; idRenta: number; datos: PagoDatos }) => ({
			id: 1,
			idRenta: 5,
			fecha: '2026-08-02',
			monto: '200000.00',
			metodoPago: 'Efectivo',
			concepto: 'Abono renta',
			observaciones: null,
			usuario: 'admin'
		}));
		tauri.register('registrar_pago_renta', pagar);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		await fireEvent.click(screen.getByTitle('Registrar pago'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Registrar pago — renta #5');

		await fireEvent.input(screen.getByPlaceholderText('Ej: 200000'), {
			target: { value: '200000' }
		});

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Registrar pago' }));

		await waitFor(() => expect(pagar).toHaveBeenCalledTimes(1));
		const args = pagar.mock.calls[0][0] as { sessionId: string; idRenta: number; datos: PagoDatos };
		expect(args.idRenta).toBe(5);
		expect(args.datos.monto).toBe('200000');
	});

	it('registra una inspección de salida', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 5 })]);
		const inspeccionar = vi.fn((_args: { sessionId: string; idRenta: number; datos: InspeccionDatos }) => ({
			id: 1,
			idRenta: 5,
			tipo: 'Salida',
			fecha: '2026-08-01',
			kilometraje: '42000',
			nivelGasolina: 'Lleno',
			limpieza: 'Limpio',
			tieneRepuesto: true,
			tieneGatoCruceta: true,
			tieneKitCarretera: true,
			tieneDocumentos: true,
			danosCarroceria: null,
			observaciones: null
		}));
		tauri.register('registrar_inspeccion_renta', inspeccionar);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		await fireEvent.click(screen.getByTitle('Registrar inspección'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Inspección de Salida — renta #5');

		// El km de salida se autocompleta desde la renta
		await fireEvent.input(screen.getByDisplayValue('42000'), {
			target: { value: '42100' }
		});
		await fireEvent.input(screen.getByPlaceholderText('Describir golpes, rayones...'), {
			target: { value: 'Rayón en puerta izquierda' }
		});

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Registrar inspección' }));

		await waitFor(() => expect(inspeccionar).toHaveBeenCalledTimes(1));
		const args = inspeccionar.mock.calls[0][0] as { sessionId: string; idRenta: number; datos: InspeccionDatos };
		expect(args.idRenta).toBe(5);
		expect(args.datos.tipo).toBe('Salida');
		expect(args.datos.kilometraje).toBe('42100');
		expect(args.datos.danosCarroceria).toBe('Rayón en puerta izquierda');
	});

	it('elimina una renta tras confirmar', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 3 })]);
		const eliminar = vi.fn((_args: { sessionId: string; id: number }) => undefined);
		tauri.register('eliminar_renta', eliminar);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		await fireEvent.click(screen.getByTitle('Eliminar'));
		const dialogo = await screen.findByRole('dialog');
		expect(dialogo).toHaveTextContent('Eliminar renta');

		await fireEvent.click(within(dialogo).getByRole('button', { name: 'Eliminar' }));

		await waitFor(() => expect(eliminar).toHaveBeenCalledTimes(1));
		const args = eliminar.mock.calls[0][0] as { sessionId: string; id: number };
		expect(args.id).toBe(3);
	});

	it('abre el documento imprimible con el detalle completo (pagos e inspecciones)', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 1 })]);
		// El listado no incluye pagos/inspecciones: la impresión obtiene el detalle
		tauri.register('obtener_renta', () =>
			renta({
				id: 1,
				pagos: [
					{
						id: 1,
						idRenta: 1,
						fecha: '2026-08-02',
						monto: '200000.00',
						metodoPago: 'Efectivo',
						concepto: 'Abono renta',
						observaciones: null,
						usuario: 'admin'
					}
				],
				inspecciones: [
					{
						id: 1,
						idRenta: 1,
						tipo: 'Salida',
						fecha: '2026-08-01',
						kilometraje: '42000',
						nivelGasolina: 'Lleno',
						limpieza: 'Limpio',
						tieneRepuesto: true,
						tieneGatoCruceta: true,
						tieneKitCarretera: true,
						tieneDocumentos: true,
						danosCarroceria: null,
						observaciones: null
					}
				]
			})
		);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		await fireEvent.click(screen.getByTitle('Imprimir orden de renta'));

		expect(await screen.findByRole('dialog')).toHaveTextContent('Orden de renta #0001');
		// El documento imprimible muestra el desglose completo
		expect(screen.getByText('ORDEN DE RENTA')).toBeInTheDocument();
		expect(screen.getByText(/TOTAL/)).toBeInTheDocument();
		expect(screen.getByText('DINAMO RENT A CAR')).toBeInTheDocument();
		// Pagos e inspecciones (que solo vienen con obtener_renta)
		expect(screen.getByText('Abono renta')).toBeInTheDocument();
		expect(screen.getByText('Inspección de salida')).toBeInTheDocument();
	});

	it('abre el contrato como documento independiente (papel Carta)', async () => {
		tauri.register('listar_rentas', () => [renta({ id: 1 })]);
		tauri.register('obtener_renta', () =>
			renta({
				id: 1,
				pagos: [],
				inspecciones: [
					{
						id: 1,
						idRenta: 1,
						tipo: 'Salida',
						fecha: '2026-08-01',
						kilometraje: '42000',
						nivelGasolina: 'Lleno',
						limpieza: 'Limpio',
						tieneRepuesto: true,
						tieneGatoCruceta: true,
						tieneKitCarretera: true,
						tieneDocumentos: true,
						danosCarroceria: null,
						observaciones: null
					}
				]
			})
		);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');

		// 1) Abrir la orden
		await fireEvent.click(screen.getByTitle('Imprimir orden de renta'));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Orden de renta #0001');
		// La orden muestra el número de contrato por año (2026-042) y el No. de renta (id 1):
		// el pie del documento los combina en un solo nodo de texto (el encabezado
		// reparte su texto entre <p> y <span>, por eso se verifica el pie).
		await waitFor(() => {
			expect(
				screen.getByText((c) => c.includes('Contrato 2026-042') && c.includes('Renta No. 0001'))
			).toBeInTheDocument();
		});
		// 2) La orden ya no incluye el contrato embebido
		expect(screen.queryByText(/ANEXO DE CONTRATO/)).not.toBeInTheDocument();

		// 3) Pasar al contrato independiente
		await fireEvent.click(screen.getByRole('button', { name: /Ver contrato/ }));
		expect(await screen.findByRole('dialog')).toHaveTextContent('Contrato de renta #0001');
		// El contrato trae su encabezado legal y cláusulas
		expect(screen.getByText(/ANEXO DE CONTRATO DE ALQUILER/)).toBeInTheDocument();
		expect(screen.getByText(/ENTRE LOS SUSCRITOS/)).toBeInTheDocument();
		expect(screen.getByText(/CLÁUSULA PRIMERA/)).toBeInTheDocument();
		expect(screen.getByText(/PÓLIZA DE SEGURO POR LUCRO CESANTE/)).toBeInTheDocument();
		// El número de contrato es la secuencia por año (2026-042), independiente del id (1),
		// con el mismo formato que el listado y la orden
		expect(screen.getByText(/CONTRATO Nº: 2026-042/)).toBeInTheDocument();
	});

	it('filtra por estado con el selector', async () => {
		const listar = vi.fn((_args: { sessionId: string; estado: string | null; placa: string | null }) => [renta()]);
		tauri.register('listar_rentas', listar);

		render(RentasPage);
		await screen.findByText('Cliente de Prueba');
		expect(listar).toHaveBeenCalledTimes(1);

		const select = screen.getByLabelText('Filtrar por estado');
		await fireEvent.change(select, { target: { value: 'Cerrada' } });

		await waitFor(() => expect(listar).toHaveBeenCalledTimes(2), { timeout: 2000 });
		const args = listar.mock.calls[1][0] as { sessionId: string; estado: string | null };
		expect(args.estado).toBe('Cerrada');
	});
});
