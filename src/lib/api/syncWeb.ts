// syncWeb.ts — Cliente de sincronización con la plataforma Web (Neon PostgreSQL)
import { clienteApi } from './clientes';
import { reservaApi } from './reservas';

export interface WebReservation {
	id: string;
	code: string;
	status: string;
	totalAmount: number;
	blockingAmount: number;
	days: number;
	pickupDate: string;
	returnDate: string;
	pickupTime: string;
	returnTime: string;
	pickupLocation: string;
	returnLocation: string;
	insurancePlan: string;
	synced: boolean;
	desktopRef: string | null;
	customer: {
		id: string;
		docNumber: string;
		docType?: string;
		fullName: string;
		names?: string;
		lastnames?: string;
		email: string;
		phone: string;
		license?: string | null;
		hotel?: string | null;
		desktopId?: number | null;
	};
	vehicle: {
		id: string;
		name: string;
		plate: string | null;
		image: string;
	};
	payment?: {
		status: string;
		cardBrand?: string;
		cardLast4?: string;
	} | null;
}

export interface SyncWebResult {
	ok: boolean;
	totalPendientes: number;
	importadas: number;
	clientesNuevos: number;
	clientesReutilizados: number;
	errores: string[];
}

export const DEFAULT_WEB_SYNC_URL = 'http://localhost:5174/api/sync';

export const syncWebApi = {
	/**
	 * Consulta cuántas reservas web están pendientes de importar en mostrador
	 */
	consultarPendientes: async (
		syncUrl = DEFAULT_WEB_SYNC_URL
	): Promise<{ ok: boolean; count: number; reservations: WebReservation[] }> => {
		try {
			const res = await fetch(`${syncUrl}?pending=true`);
			const data = await res.json();
			if (data?.ok) {
				return {
					ok: true,
					count: data.count || 0,
					reservations: data.reservations || []
				};
			}
			return { ok: false, count: 0, reservations: [] };
		} catch {
			return { ok: false, count: 0, reservations: [] };
		}
	},

	/**
	 * Descarga e importa reservas web garantizando CERO DUPLICADOS de clientes en Firebird
	 */
	sincronizarReservas: async (
		sessionId: string,
		syncUrl = DEFAULT_WEB_SYNC_URL
	): Promise<SyncWebResult> => {
		const resultado: SyncWebResult = {
			ok: true,
			totalPendientes: 0,
			importadas: 0,
			clientesNuevos: 0,
			clientesReutilizados: 0,
			errores: []
		};

		try {
			const pendientesRes = await syncWebApi.consultarPendientes(syncUrl);
			if (!pendientesRes.ok || pendientesRes.reservations.length === 0) {
				return resultado;
			}

			resultado.totalPendientes = pendientesRes.reservations.length;

			for (const webR of pendientesRes.reservations) {
				try {
					// ── PASO 1: Deduplicación estricta de cliente en Firebird ──
					const cleanDoc = webR.customer.docNumber.trim();
					let idCliente: number | null = null;

					// Buscar si ya existe en clientes por documento
					const busqueda = await clienteApi.listar(sessionId, cleanDoc);
					const clienteExistente = busqueda.find((c) => c.cliente.noDoc?.trim() === cleanDoc);

					if (clienteExistente) {
						idCliente = clienteExistente.cliente.id;
						resultado.clientesReutilizados++;
					} else {
						// Crear nuevo cliente con cifrado PII automático
						const nuevoCliente = await clienteApi.crear(sessionId, {
							tipoDoc: webR.customer.docType || 'CC',
							noDoc: cleanDoc,
							nombres: webR.customer.names || webR.customer.fullName,
							apellidos: webR.customer.lastnames || '',
							celular: webR.customer.phone,
							email: webR.customer.email,
							hotel: webR.customer.hotel || undefined,
							noLicencia: webR.customer.license || undefined,
							estado: 'Activo'
						});
						idCliente = nuevoCliente.cliente.id;
						resultado.clientesNuevos++;
					}

					// ── PASO 2: Inserción de la reserva en Firebird ──
					const fechaRec = webR.pickupDate.includes('T')
						? webR.pickupDate.split('T')[0]
						: webR.pickupDate;
					const fechaRet = webR.returnDate.includes('T')
						? webR.returnDate.split('T')[0]
						: webR.returnDate;

					const obs =
						`[ORIGEN: WEB - ${webR.code}]\n` +
						`Pago: PAGADA ONLINE Place to Pay (${webR.payment?.cardBrand || 'Tarjeta'} •••• ${webR.payment?.cardLast4 || ''})\n` +
						`Vehículo Solicitado: ${webR.vehicle.name}\n` +
						`Cobertura: ${webR.insurancePlan === 'TOTAL' ? 'Total Cero Deducible' : 'Básica Legal'}\n` +
						`Garantía a bloquear en mostrador: $${webR.blockingAmount.toLocaleString('es-CO')}`;

					const reservaCreada = await reservaApi.crear(sessionId, {
						idCliente,
						nombreCliente: webR.customer.fullName,
						fechaRecogida: fechaRec,
						horaRecogida: webR.pickupTime || '10:00',
						ubicacionRecogida: webR.pickupLocation,
						fechaRetorno: fechaRet,
						horaRetorno: webR.returnTime || '10:00',
						ubicacionRetorno: webR.returnLocation,
						categoriaVehiculo: webR.vehicle.name,
						placaAsignada: webR.vehicle.plate || undefined,
						diasCalculados: webR.days,
						horasExtras: 0,
						valorDia: String(Math.round(webR.totalAmount / (webR.days || 1))),
						valorHoraAdic: '0',
						costoLavado: '0',
						abono: String(webR.totalAmount),
						total: String(webR.totalAmount),
						estado: 'Confirmada',
						observaciones: obs
					});

					// ── PASO 3: Notificar a la Web la confirmación con IDs de mostrador ──
					await fetch(syncUrl, {
						method: 'POST',
						headers: { 'Content-Type': 'application/json' },
						body: JSON.stringify({
							entity: 'RESERVATION',
							entityId: webR.id,
							desktopRef: `RES-FB-${reservaCreada.id}`,
							desktopCustomerId: idCliente
						})
					});

					resultado.importadas++;
				} catch (err) {
					console.error(`Error importando reserva web ${webR.code}:`, err);
					resultado.errores.push(`${webR.code}: ${(err as Error).message}`);
				}
			}

			return resultado;
		} catch (e) {
			resultado.ok = false;
			resultado.errores.push((e as Error).message);
			return resultado;
		}
	}
};
