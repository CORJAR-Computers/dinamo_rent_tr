// GENERATED FILE — NO EDITAR A MANO.
// Generado por `cargo run --features dev --bin gen_devguard`
// (clasificación conductual: sondas serde reales sobre los DTOs de entrada).
// El test devGuard.sync.test.ts valida que este archivo esté al día.

export const MAPA_GENERADO: Record<string, { numeros: readonly string[]; strings: readonly string[] }> = {
	actualizar_auto: {
		// DTO: AutoDatos
		numeros: ['kilometraje', 'proximoAceite', 'proximoFrenos'],
		strings: ['costoFijoMensual'],
	},
	actualizar_cliente: {
		// DTO: ClienteDatos
		numeros: [],
		strings: [],
	},
	actualizar_comparendo: {
		// DTO: ComparendoDatos
		numeros: ['idCliente', 'idRenta'],
		strings: ['monto'],
	},
	actualizar_gasto: {
		// DTO: GastoDatos
		numeros: [],
		strings: ['monto'],
	},
	actualizar_mantenimiento: {
		// DTO: MantenimientoDatos
		numeros: ['kmProximoCambioAceite'],
		strings: ['costo'],
	},
	actualizar_renta: {
		// DTO: RentaDatos
		numeros: ['diasCalculados', 'horasExtras', 'idCliente', 'idReserva'],
		strings: ['abono', 'comision', 'costoCables', 'costoDomicilio', 'costoInversor', 'costoLavado', 'costoRetorno', 'costoSilla', 'descuento', 'impuestos', 'kmSalida', 'saldoPendiente', 'subtotal', 'total', 'valorDia', 'valorDiaExtra', 'valorGasolina', 'valorHoraExtra', 'valorNeto'],
	},
	actualizar_reserva: {
		// DTO: ReservaDatos
		numeros: ['diasCalculados', 'horasExtras', 'idCliente'],
		strings: ['abono', 'costoLavado', 'total', 'valorDia', 'valorHoraAdic'],
	},
	actualizar_usuario: {
		// DTO: UsuarioDatosActualizar
		numeros: [],
		strings: [],
	},
	cerrar_renta: {
		// DTO: RentaCierreDatos
		numeros: ['diasCalculados', 'horasExtras'],
		strings: ['descuento', 'kmFinal', 'valorDia', 'valorDiaExtra', 'valorHoraExtra'],
	},
	crear_auto: {
		// DTO: AutoDatos
		numeros: ['kilometraje', 'proximoAceite', 'proximoFrenos'],
		strings: ['costoFijoMensual'],
	},
	crear_cliente: {
		// DTO: ClienteDatos
		numeros: [],
		strings: [],
	},
	crear_comparendo: {
		// DTO: ComparendoDatos
		numeros: ['idCliente', 'idRenta'],
		strings: ['monto'],
	},
	crear_gasto: {
		// DTO: GastoDatos
		numeros: [],
		strings: ['monto'],
	},
	crear_mantenimiento: {
		// DTO: MantenimientoDatos
		numeros: ['kmProximoCambioAceite'],
		strings: ['costo'],
	},
	crear_renta: {
		// DTO: RentaDatos
		numeros: ['diasCalculados', 'horasExtras', 'idCliente', 'idReserva'],
		strings: ['abono', 'comision', 'costoCables', 'costoDomicilio', 'costoInversor', 'costoLavado', 'costoRetorno', 'costoSilla', 'descuento', 'impuestos', 'kmSalida', 'saldoPendiente', 'subtotal', 'total', 'valorDia', 'valorDiaExtra', 'valorGasolina', 'valorHoraExtra', 'valorNeto'],
	},
	crear_reserva: {
		// DTO: ReservaDatos
		numeros: ['diasCalculados', 'horasExtras', 'idCliente'],
		strings: ['abono', 'costoLavado', 'total', 'valorDia', 'valorHoraAdic'],
	},
	crear_usuario: {
		// DTO: UsuarioDatos
		numeros: [],
		strings: [],
	},
	editar_renta_cerrada: {
		// DTO: RentaCierreEditDatos
		numeros: ['diasCalculados', 'horasExtras'],
		strings: ['descuento', 'valorDia', 'valorDiaExtra', 'valorHoraExtra'],
	},
	extender_renta: {
		// DTO: ExtensionDatos
		numeros: ['cantidad'],
		strings: ['valor'],
	},
	guardar_empresa: {
		// DTO: EmpresaConfigDatos
		numeros: [],
		strings: [],
	},
	registrar_inspeccion_renta: {
		// DTO: InspeccionDatos
		numeros: [],
		strings: ['kilometraje'],
	},
	registrar_pago_renta: {
		// DTO: PagoDatos
		numeros: [],
		strings: ['monto'],
	},
} as const;
