// format.ts — Formato de moneda COP y fechas

const currencyFormatter = new Intl.NumberFormat('es-CO', {
	style: 'currency',
	currency: 'COP',
	minimumFractionDigits: 0,
	maximumFractionDigits: 0
});

const currencyFormatterDecimal = new Intl.NumberFormat('es-CO', {
	style: 'currency',
	currency: 'COP',
	minimumFractionDigits: 2,
	maximumFractionDigits: 2
});

/** Formatea un monto como moneda COP (sin decimales por defecto) */
export function formatCOP(value: number | string | null | undefined, decimals = false): string {
	if (value === null || value === undefined || value === '') return '$0';
	const num = typeof value === 'string' ? parseFloat(value) : value;
	if (Number.isNaN(num)) return '$0';
	return decimals ? currencyFormatterDecimal.format(num) : currencyFormatter.format(num);
}

const dateFormatter = new Intl.DateTimeFormat('es-CO', {
	year: 'numeric',
	month: 'short',
	day: 'numeric'
});

const dateTimeFormatter = new Intl.DateTimeFormat('es-CO', {
	year: 'numeric',
	month: 'short',
	day: 'numeric',
	hour: '2-digit',
	minute: '2-digit'
});

/** Formatea una fecha ISO (YYYY-MM-DD) o Date en formato corto es-CO */
export function formatDate(value: string | Date | null | undefined): string {
	if (!value) return '—';
	const d = typeof value === 'string' ? new Date(value) : value;
	if (Number.isNaN(d.getTime())) return '—';
	return dateFormatter.format(d);
}

export function formatDateTime(value: string | Date | null | undefined): string {
	if (!value) return '—';
	const d = typeof value === 'string' ? new Date(value) : value;
	if (Number.isNaN(d.getTime())) return '—';
	return dateTimeFormatter.format(d);
}

/** Trunca texto largo con elipsis */
export function truncate(text: string, max = 40): string {
	if (!text || text.length <= max) return text ?? '';
	return text.slice(0, max - 1) + '…';
}

/** Formatea el número de contrato anual: 2026-001 (secuencia reiniciada por año) */
export function formatContrato(anio: number | null | undefined, secuencia: number | null | undefined): string {
	if (anio == null || secuencia == null) return '—';
	return `${anio}-${String(secuencia).padStart(3, '0')}`;
}
