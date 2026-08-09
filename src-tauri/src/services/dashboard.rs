//! services/dashboard.rs — KPIs del Dashboard (puerto parcial de dashboard_service.py)
//!
//! Agrega datos de los módulos ya portados (autos, clientes, alertas).
//! Rentas/pagos se integrarán en Fase 4 (el conteo de rentas activas ya se expone).

use rsfbclient::Queryable;
use serde::Serialize;

use crate::core::crypto::PiiCipher;
use crate::core::error::AppError;
use crate::repositories::cliente::Cliente;
use crate::services::AppState;
use crate::services::auto::{AlertaVencimiento, AutoService};
use crate::services::cliente::ClienteService;

/// Conteo por estado de vehículo
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadoCount {
    pub estado: String,
    pub total: i64,
}

/// Datos agregados para la pantalla de inicio
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total_autos: i64,
    pub autos_por_estado: Vec<EstadoCount>,
    pub total_clientes: i64,
    pub clientes_recientes: Vec<Cliente>,
    pub alertas: Vec<AlertaVencimiento>,
    pub rentas_activas: i64,
    /// false si la clave PII no está configurada (datos legacy Fernet ocultos)
    pub pii_key_configurada: bool,
}

pub struct DashboardService;

impl DashboardService {
    pub fn get_dashboard_data(state: &AppState) -> Result<DashboardData, AppError> {
        let mut conn = state.pool.get()?;

        let total_autos = AutoService::contar(&mut conn)?;
        let autos_por_estado = AutoService::contar_por_estado(&mut conn)?
            .into_iter()
            .map(|(estado, total)| EstadoCount { estado, total })
            .collect();
        let total_clientes = ClienteService::contar(&mut conn)?;
        let cipher = PiiCipher::new(&state.pii_key());
        let clientes_recientes = ClienteService::recientes(&mut conn, &cipher, 6)?
            .into_iter()
            .map(|c| c.cliente)
            .collect();
        let alertas = AutoService::alertas_vencimiento(&mut conn, &state.config)?;
        // Conteo parcial de rentas activas (repo de rentas llega en Fase 4)
        let rentas_activas: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM rentas WHERE estado = 'Activo'",
            (),
        )?;

        Ok(DashboardData {
            total_autos,
            autos_por_estado,
            total_clientes,
            clientes_recientes,
            alertas,
            rentas_activas: rentas_activas.map(|(c,)| c).unwrap_or(0),
            pii_key_configurada: state.pii_key_configurada(),
        })
    }
}
