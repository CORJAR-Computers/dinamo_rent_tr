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
	signatureDataUrl?: string | null;
	docPhotoUrl?: string | null;
	licensePhotoUrl?: string | null;
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

export const DEFAULT_WEB_SYNC_URL = 'https://dinamo-rent-a-car.vercel.app/api/sync';
export const DEFAULT_WEB_SYNC_TOKEN = 'dinamo-desktop-sync-secret-2026';

function getSyncConfig() {
	if (typeof window !== 'undefined') {
		const storedUrl = localStorage.getItem('dinamo.web.sync_url');
		const storedToken = localStorage.getItem('dinamo.web.sync_token');
		return {
			url: storedUrl || DEFAULT_WEB_SYNC_URL,
			token: storedToken || DEFAULT_WEB_SYNC_TOKEN
		};
	}
	return {
		url: DEFAULT_WEB_SYNC_URL,
		token: DEFAULT_WEB_SYNC_TOKEN
	};
}

export const syncWebApi = {
	/**
	 * Consulta cuántas reservas web están pendientes de importar en mostrador
	 */
	consultarPendientes: async (
		customUrl?: string,
		customToken?: string
	): Promise<{ ok: boolean; count: number; reservations: WebReservation[] }> => {
		try {
			const config = getSyncConfig();
			const syncUrl = customUrl || config.url;
			const syncToken = customToken || config.token;

			const res = await fetch(`${syncUrl}?pending=true`, {
				headers: {
					'x-sync-token': syncToken
				}
			});
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
		customUrl?: string,
		customToken?: string
	): Promise<SyncWebResult> => {
		const config = getSyncConfig();
		const syncUrl = customUrl || config.url;
		const syncToken = customToken || config.token;

		const resultado: SyncWebResult = {
			ok: true,
			totalPendientes: 0,
			importadas: 0,
			clientesNuevos: 0,
			clientesReutilizados: 0,
			errores: []
		};

		try {
			const pendientesRes = await syncWebApi.consultarPendientes(syncUrl, syncToken);
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

					const tieneDoc = webR.docPhotoUrl ? 'SÍ (Digitalizado)' : 'NO';
					const tieneLic = webR.licensePhotoUrl ? 'SÍ (Digitalizada)' : 'NO';
					const tieneFirma = webR.signatureDataUrl ? 'SÍ (Firmado digitalmente)' : 'NO';

					const obs =
						`[ORIGEN: WEB - ${webR.code}]\n` +
						`Pago: PAGADA ONLINE Place to Pay (${webR.payment?.cardBrand || 'Tarjeta'} •••• ${webR.payment?.cardLast4 || ''})\n` +
						`Vehículo Solicitado: ${webR.vehicle.name}\n` +
						`Cobertura: ${webR.insurancePlan === 'TOTAL' ? 'Total Cero Deducible' : 'Básica Legal'}\n` +
						`Garantía a bloquear en mostrador: $${webR.blockingAmount.toLocaleString('es-CO')}\n` +
						`Documento: ${tieneDoc} | Licencia: ${tieneLic} | Firma: ${tieneFirma}`;

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
						headers: {
							'Content-Type': 'application/json',
							'x-sync-token': syncToken
						},
						body: JSON.stringify({
							entity: 'RESERVATION',
							entityId: webR.id,
							desktopRef: `RES-FB-${reservaCreada.id}`,
							desktopCustomerId: idCliente
						})
					});

					// Guardar en caché local los documentos digitales para consulta inmediata en mostrador
					if (
						typeof window !== 'undefined' &&
						(webR.docPhotoUrl || webR.licensePhotoUrl || webR.signatureDataUrl)
					) {
						try {
							localStorage.setItem(
								`dinamo.docs.${webR.code}`,
								JSON.stringify({
									code: webR.code,
									docPhotoUrl: webR.docPhotoUrl,
									licensePhotoUrl: webR.licensePhotoUrl,
									signatureDataUrl: webR.signatureDataUrl,
									customer: {
										fullName: webR.customer.fullName,
										docNumber: webR.customer.docNumber,
										docType: webR.customer.docType,
										license: webR.customer.license,
										phone: webR.customer.phone,
										email: webR.customer.email
									}
								})
							);
						} catch {
							// Ignorar cuota de localStorage excedida
						}
					}

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
	},

	/**
	 * Obtiene los documentos digitalizados (Cédula, Licencia, Firma) de una reserva web.
	 * Primero busca en la caché local; si no están, los descarga bajo demanda desde la Web API.
	 */
	obtenerDocumentosReserva: async (
		code: string,
		customUrl?: string,
		customToken?: string
	): Promise<{
		ok: boolean;
		code?: string;
		docPhotoUrl?: string | null;
		licensePhotoUrl?: string | null;
		signatureDataUrl?: string | null;
		customer?: {
			fullName: string;
			docNumber: string;
			docType?: string;
			license?: string | null;
			phone: string;
			email: string;
		};
		error?: string;
	}> => {
		const cleanCode = code.trim();
		if (!cleanCode) return { ok: false, error: 'Código de reserva no especificado' };

		// 1. Revisar caché local
		if (typeof window !== 'undefined') {
			try {
				const cache = localStorage.getItem(`dinamo.docs.${cleanCode}`);
				if (cache) {
					const parsed = JSON.parse(cache);
					return { ok: true, ...parsed };
				}
			} catch {
				// error leyendo cache, continuar con fetch
			}
		}

		// 2. Consultar a la Web API
		try {
			const config = getSyncConfig();
			const syncUrl = customUrl || config.url;
			const syncToken = customToken || config.token;

			const res = await fetch(`${syncUrl}?code=${encodeURIComponent(cleanCode)}`, {
				headers: {
					'x-sync-token': syncToken
				}
			});

			const data = await res.json();
			if (data?.ok && data.reservation) {
				const r = data.reservation;
				const resultado = {
					ok: true,
					code: r.code,
					docPhotoUrl: r.docPhotoUrl,
					licensePhotoUrl: r.licensePhotoUrl,
					signatureDataUrl: r.signatureDataUrl,
					customer: r.customer
				};

				// Guardar en caché local
				if (typeof window !== 'undefined') {
					try {
						localStorage.setItem(`dinamo.docs.${cleanCode}`, JSON.stringify(resultado));
					} catch {
						// ignorar fallo de cuota
					}
				}

				return resultado;
			}
			return { ok: false, error: data?.error || 'No se encontraron documentos para esta reserva' };
		} catch (err) {
			return { ok: false, error: (err as Error).message };
		}
	}
};
