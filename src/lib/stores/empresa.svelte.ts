// empresa.svelte.ts — Store de la empresa (setup inicial, branding dinámico)
//
// Carga la vista pública (nombre + logo) para el login y el menú lateral, y la
// configuración completa (datos + logo) para la página /empresa y las
// impresiones (ContratoRenta, OrdenRenta, OrdenReserva, OrdenComparendo).
// La fuente de verdad es EMPRESA_CONFIG (backend); ante error o config vacía
// se conserva el branding estático por defecto (fallbacks ajustables por marca:
// el clon comercial usa DynaRent).

import { empresaApi, type EmpresaConfig } from '$lib/api';

/** Branding por defecto cuando la empresa aún no configuró nada. */
export const FALLBACK_NOMBRE = 'Dinamo Rent';
export const FALLBACK_LOGO = '/LogoDinamo.png';
export const FALLBACK_NIT = '900694866-3';
export const FALLBACK_TELEFONO = '301 613 85 30 • 321 839 44 22';
export const FALLBACK_EMAIL = 'info@dinamorentacar.com';
export const FALLBACK_WEB = 'www.dinamorentacar.com';
export const FALLBACK_DIRECCION = 'Carrera 2 #70-53, Barrio Crespo, Cartagena, Colombia';

/** Añade +57 a los celulares colombianos (10 dígitos que empiezan por 3) de un
 *  texto de contacto separado por • | , ; - , sin duplicar si ya tienen prefijo. */
function conPrefijo57(tel: string): string {
	return tel
		.split(/\s*[•|,;-]\s*/)
		.map((t) => t.trim())
		.filter(Boolean)
		.map((t) => {
			const digitos = t.replace(/\D/g, '');
			if (digitos.length === 10 && digitos.startsWith('3') && !t.includes('+57')) {
				return `+57 ${t}`;
			}
			return t;
		})
		.join(' • ');
}
export const FALLBACK_CIUDAD = 'CARTAGENA';

class EmpresaStore {
	// Vista pública (login / menú lateral)
	nombre = $state<string | null>(null);
	logo = $state<string | null>(null);
	cargado = $state(false);

	// Datos completos (página /empresa + impresiones)
	nit = $state<string | null>(null);
	direccion = $state<string | null>(null);
	telefono = $state<string | null>(null);
	email = $state<string | null>(null);
	web = $state<string | null>(null);
	ciudad = $state<string | null>(null);
	completaCargada = $state(false);

	// ── Getters con fallback estático ──
	get nombreMostrar(): string {
		return this.nombre?.trim() || FALLBACK_NOMBRE;
	}

	get logoSrc(): string {
		return this.logo?.trim() || FALLBACK_LOGO;
	}

	get nitMostrar(): string {
		return this.nit?.trim() || FALLBACK_NIT;
	}

	get telefonoMostrar(): string {
		return conPrefijo57(this.telefono?.trim() || FALLBACK_TELEFONO);
	}

	get emailMostrar(): string {
		return this.email?.trim() || FALLBACK_EMAIL;
	}

	get webMostrar(): string {
		return this.web?.trim() || FALLBACK_WEB;
	}

	get direccionMostrar(): string {
		return this.direccion?.trim() || FALLBACK_DIRECCION;
	}

	/** Ciudad de la empresa: primero la configurada en el setup (/empresa);
	 *  si no, se deriva de la dirección (p. ej. la penúltima parte de
	 *  "Carrera 2 #70-53, Barrio Crespo, Cartagena, Colombia" → CARTAGENA);
	 *  y como último recurso el fallback de la marca. */
	get ciudadMostrar(): string {
		const propia = this.ciudad?.trim();
		if (propia) return propia.toUpperCase();
		const d = this.direccion?.trim();
		if (d) {
			const partes = d.split(',').map((s) => s.trim()).filter(Boolean);
			const ciudad = partes.length >= 2 ? partes[partes.length - 2] : partes[partes.length - 1];
			if (ciudad) return ciudad.toUpperCase();
		}
		return FALLBACK_CIUDAD;
	}

	/** Carga la vista pública (best-effort: ante error conserva el fallback). */
	async cargarPublica(): Promise<void> {
		if (this.cargado) return;
		try {
			const cfg = await empresaApi.publica();
			this.nombre = cfg.nombre;
			this.logo = cfg.logo;
		} catch (e) {
			console.warn('No se pudo cargar la configuración de la empresa:', e);
		} finally {
			this.cargado = true;
		}
	}

	/** Carga la configuración completa (requiere sesión; usada por las impresiones). */
	async cargarCompleta(sessionId: string): Promise<void> {
		if (this.completaCargada) return;
		try {
			const cfg = await empresaApi.obtener(sessionId);
			this.aplicar(cfg);
		} catch (e) {
			console.warn('No se pudo cargar la configuración completa de la empresa:', e);
		} finally {
			this.completaCargada = true;
		}
	}

	/** Refresca en caliente tras guardar desde la página /empresa. */
	actualizar(cfg: EmpresaConfig): void {
		this.aplicar(cfg);
	}

	private aplicar(cfg: EmpresaConfig): void {
		this.nombre = cfg.nombre;
		this.logo = cfg.logo;
		this.nit = cfg.nit;
		this.direccion = cfg.direccion;
		this.telefono = cfg.telefono;
		this.email = cfg.email;
		this.web = cfg.web;
		this.ciudad = cfg.ciudad;
		this.cargado = true;
		this.completaCargada = true;
	}
}

export const empresa = new EmpresaStore();
