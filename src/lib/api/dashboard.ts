// dashboard.ts — Datos del Dashboard
import { invokeCmd } from './base';
import type { AlertaVencimiento } from './autos';
import type { Cliente } from './clientes';

/** Datos del Dashboard (services/dashboard.rs) */
export interface DashboardData {
	totalAutos: number;
	autosPorEstado: { estado: string; total: number }[];
	totalClientes: number;
	clientesRecientes: Cliente[];
	alertas: AlertaVencimiento[];
	rentasActivas: number;
	piiKeyConfigurada: boolean;
}

export const dashboardApi = {
	getData: (sessionId: string) => invokeCmd<DashboardData>('get_dashboard_data', { sessionId })
};
