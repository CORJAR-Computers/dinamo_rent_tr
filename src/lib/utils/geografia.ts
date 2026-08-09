/**
 * geografia.ts — Catálogos base de países, departamentos y ciudades para los
 * selects del formulario de cliente. Las opciones mostradas se construyen como
 * «set base + valores ya usados en la BD», así el operador puede elegir de la
 * lista o agregar un valor nuevo (que quedará disponible la próxima vez).
 */

/** Países más frecuentes en la operación (orden de uso probable). */
export const PAISES_BASE: string[] = [
	'Colombia',
	'Estados Unidos',
	'Venezuela',
	'Ecuador',
	'Perú',
	'Panamá',
	'México',
	'Brasil',
	'Argentina',
	'Chile',
	'España',
	'Canadá',
	'Costa Rica',
	'El Salvador',
	'Guatemala',
	'Honduras',
	'Nicaragua',
	'Uruguay',
	'Paraguay',
	'Bolivia',
	'República Dominicana',
	'Puerto Rico',
	'Francia',
	'Alemania',
	'Italia',
	'Reino Unido',
	'China',
	'Japón',
	'Israel'
];

/** Los 32 departamentos de Colombia + Bogotá D.C. */
export const DEPARTAMENTOS_COLOMBIA: string[] = [
	'Amazonas',
	'Antioquia',
	'Arauca',
	'Atlántico',
	'Bolívar',
	'Boyacá',
	'Caldas',
	'Caquetá',
	'Casanare',
	'Cauca',
	'Cesar',
	'Chocó',
	'Córdoba',
	'Cundinamarca',
	'Guainía',
	'Guaviare',
	'Huila',
	'La Guajira',
	'Magdalena',
	'Meta',
	'Nariño',
	'Norte de Santander',
	'Putumayo',
	'Quindío',
	'Risaralda',
	'San Andrés y Providencia',
	'Santander',
	'Sucre',
	'Tolima',
	'Valle del Cauca',
	'Vaupés',
	'Vichada',
	'Bogotá D.C.'
];

/** Ciudades principales de Colombia. */
export const CIUDADES_COLOMBIA: string[] = [
	'Bogotá',
	'Medellín',
	'Cali',
	'Barranquilla',
	'Cartagena',
	'Bucaramanga',
	'Pereira',
	'Manizales',
	'Cúcuta',
	'Ibagué',
	'Villavicencio',
	'Pasto',
	'Montería',
	'Neiva',
	'Armenia',
	'Sincelejo',
	'Valledupar',
	'Riohacha',
	'Popayán',
	'Tunja',
	'Santa Marta',
	'Soledad',
	'Bello',
	'Envigado',
	'Itagüí',
	'Floridablanca',
	'Girón',
	'Palmira',
	'Tuluá',
	'Yumbo',
	'Buenaventura',
	'Soacha',
	'Chía',
	'Zipaquirá',
	'Facatativá',
	'Rionegro',
	'Barrancabermeja',
	'Duitama',
	'Sogamoso',
	'Yopal',
	'Florencia',
	'San Andrés',
	'Quibdó',
	'Leticia',
	'Mocoa',
	'Inírida',
	'Puerto Carreño',
	'San José del Guaviare',
	'Mitú',
	'Arauca',
	'Turbo',
	'Apartadó',
	'Girardot'
];

/** Junta el catálogo base con valores usados; devuelve únicos, limpios y ordenados.
 * La deduplicación es insensible a mayúsculas («bogotá» y «Bogotá» cuentan como una). */
function unir(catalogo: string[], usados: (string | null | undefined)[]): string[] {
	const resultado: string[] = [];
	const vistos = new Set<string>();
	const agregar = (v: string | null | undefined) => {
		const t = (v ?? '').trim();
		if (!t) return;
		const clave = t.toLowerCase();
		if (vistos.has(clave)) return;
		vistos.add(clave);
		resultado.push(t);
	};
	for (const v of catalogo) agregar(v);
	for (const v of usados) agregar(v);
	// Orden: los del catálogo base primero (orden de preferencia), luego el resto alfabético
	const extras = resultado.slice(catalogo.length).sort((a, b) => a.localeCompare(b, 'es'));
	return [...catalogo, ...extras];
}

export const geografia = {
	paises(usados: (string | null | undefined)[] = []): string[] {
		return unir(PAISES_BASE, usados);
	},
	departamentos(usados: (string | null | undefined)[] = []): string[] {
		return unir(DEPARTAMENTOS_COLOMBIA, usados);
	},
	ciudades(usados: (string | null | undefined)[] = []): string[] {
		return unir(CIUDADES_COLOMBIA, usados);
	}
};
