#![allow(linker_messages)]
//! Dinamo Rent ERP — Tauri V2 backend

pub mod commands;
pub mod core;
pub mod repositories;
pub mod services;

use std::sync::Arc;

use services::auth::AuthService;
use services::AppState;
use tauri::Emitter;
use tauri::Manager;

use crate::core::config::AppConfig;
use crate::core::security::LoginAttemptTracker;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // ── Auto-actualización ──
            // Registro del plugin updater (Windows): lee tauri.conf.json
            // (pubkey + endpoints de GitHub Releases) y permite a la UI
            // comprobar/descargar/instalar la nueva versión (UpdateDisponible.svelte).
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            // ── Configuración ──
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let (resource_dir, data_dir) = if cfg!(debug_assertions) {
                // En desarrollo usamos las carpetas del proyecto (data/ + resources/)
                (manifest_dir.join("resources"), manifest_dir.join("../data"))
            } else {
                // En producción usamos las rutas de la app
                let r = app
                    .path()
                    .resource_dir()
                    .unwrap_or_else(|_| manifest_dir.join("resources"));
                let d = app
                    .path()
                    .app_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                (r, d)
            };
            // Asegurar que el directorio de datos exista
            if let Err(e) = std::fs::create_dir_all(&data_dir) {
                log::error!("No se pudo crear data_dir {:?}: {}", data_dir, e);
            }
            let config = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest_dir));

            // ── Pool de BD Firebird Embedded ──
            let pool = crate::core::db::create_pool(&config)?;

            // ── Migraciones ──
            let migrations_dir = manifest_dir.join("migrations");
            crate::core::migrations::run_migrations(&pool, &migrations_dir)?;

            // ── Seed admin (solo si la tabla está vacía) ──
            seed_admin(&pool)?;

            // ── Estado global ──
            let tracker = LoginAttemptTracker::new(
                config.max_login_attempts,
                config.account_lockout_duration,
                config.login_rate_limit_window,
                config.max_login_attempts_in_window,
            );
            let state = AppState {
                pool: pool.clone(),
                sessions: std::sync::Mutex::new(crate::core::rbac::SessionStore::new(
                    config.session_timeout,
                )),
                login_tracker: std::sync::Mutex::new(tracker),
                config: config.clone(),
                pii_key: std::sync::Mutex::new(config.db_encryption_key.clone()),
            };
            app.manage(state);

            // ── Estado en memoria del Agente SIMIT (comparendos automáticos) ──
            let simit_estado = std::sync::Arc::new(services::simit::EstadoAgenteSimit::default());
            app.manage(services::simit::EstadoAgenteSimitManaged(simit_estado.clone()));
            // Flag de frontend listo para el diálogo de confirmación de cierre
            // (evita bloquear la X antes de que el webview escuche el evento).
            app.manage(commands::app::FrontendListo(
                std::sync::atomic::AtomicBool::new(false),
            ));
            // Sincronizar tracker de intentos fallidos desde la BD (restaura bloqueos)
            let managed_state = app.state::<AppState>();
            AuthService::sync_tracker_from_db(&managed_state);

            // ── Agente SIMIT: consulta automática de comparendos en segundo plano ──
            // Consulta al arrancar y después cada `simit.interval_hours` (2 h).
            services::simit::spawn_scheduler(
                app.handle().clone(),
                pool.clone(),
                config.clone(),
                simit_estado,
            );

            // ── Logging ──
            // En DEBUG se loguea por defecto (stderr del terminal de dev). En
            // RELEASE también se escribe a archivo (data_dir/logs/app.log): sin
            // esto los errores de BD (que la UI solo muestra como "Error al
            // acceder a la base de datos.") quedaban sin registro en producción
            // y eran imposibles de diagnosticar. Los errores se loguean en
            // core/error.rs vía `AppError::to_payload()` (el punto por el que
            // pasan todos los errores hacia la UI).
            {
                let mut builder = tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    // 5 MB por archivo (el default de 1 MB se descartaba y podía
                    // perder justo el error a diagnosticar) + conservar rotados.
                    .max_file_size(5_000_000)
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll);
                if cfg!(debug_assertions) {
                    // Dev: consola del terminal
                } else {
                    // Prod: archivo rotativo en data_dir/logs/app.log
                    let log_dir = data_dir.join("logs");
                    let _ = std::fs::create_dir_all(&log_dir);
                    builder = builder.targets([
                        tauri_plugin_log::Target::new(
                            tauri_plugin_log::TargetKind::Folder {
                                path: log_dir,
                                file_name: Some("app".into()),
                            },
                        ),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    ]);
                }
                app.handle().plugin(builder.build())?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Confirmación de cierre: el botón X de la ventana NO cierra de
            // inmediato. Se previene el cierre y se emite `app-close-requested`
            // al frontend, que muestra «¿Está seguro de cerrar la aplicación?».
            // Si el usuario confirma, el comando `confirmar_cierre` destruye la
            // ventana con destroy() (que no vuelve a disparar CloseRequested).
            // Solo se previene cuando el frontend confirmó que escucha el evento
            // (FrontendListo); si aún no está listo, se permite el cierre normal
            // para no dejar la ventana permanentemente bloqueada.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let listo = window
                    .app_handle()
                    .try_state::<commands::app::FrontendListo>()
                    .map(|s| s.0.load(std::sync::atomic::Ordering::SeqCst))
                    .unwrap_or(false);
                if listo {
                    api.prevent_close();
                    let _ = window.emit("app-close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::confirmar_cierre,
            commands::app::app_frontend_lista,
            commands::auditoria::listar_auditoria,
            commands::auditoria::acciones_auditoria,
            commands::auditoria::usuarios_auditoria,
            commands::auth::login,
            commands::auth::logout,
            commands::auth::change_password,
            commands::auth::get_login_status,
            commands::auth::get_session,
            commands::auth::obtener_tema,
            commands::auth::guardar_tema,
            commands::auto::listar_autos,
            commands::auto::obtener_auto,
            commands::auto::crear_auto,
            commands::auto::actualizar_auto,
            commands::auto::eliminar_auto,
            commands::auto::alertas_autos,
            commands::cliente::listar_clientes,
            commands::cliente::obtener_cliente,
            commands::cliente::crear_cliente,
            commands::cliente::actualizar_cliente,
            commands::cliente::eliminar_cliente,
            commands::gasto::listar_gastos,
            commands::gasto::gastos_recientes,
            commands::gasto::obtener_gasto,
            commands::gasto::crear_gasto,
            commands::gasto::actualizar_gasto,
            commands::gasto::eliminar_gasto,
            commands::gasto::totales_gastos,
            commands::informe::informe_mensual,
            commands::mantenimiento::listar_mantenimientos,
            commands::mantenimiento::mantenimientos_recientes,
            commands::mantenimiento::obtener_mantenimiento,
            commands::mantenimiento::crear_mantenimiento,
            commands::mantenimiento::actualizar_mantenimiento,
            commands::mantenimiento::eliminar_mantenimiento,
            commands::mantenimiento::totales_mantenimiento,
            commands::mantenimiento::alertas_km_mantenimiento,
            commands::dashboard::get_dashboard_data,
            commands::empresa::empresa_publica,
            commands::empresa::obtener_empresa,
            commands::empresa::guardar_empresa,
            commands::business::get_business_lists,
            commands::comparendo::listar_comparendos,
            commands::comparendo::obtener_comparendo,
            commands::comparendo::crear_comparendo,
            commands::comparendo::actualizar_comparendo,
            commands::comparendo::marcar_pagado_comparendo,
            commands::comparendo::eliminar_comparendo,
            commands::comparendo::totales_comparendos,
            commands::simit::simit_sync_status,
            commands::simit::simit_sync_now,
            commands::reserva::listar_reservas,
            commands::reserva::proximas_reservas,
            commands::reserva::obtener_reserva,
            commands::reserva::crear_reserva,
            commands::reserva::actualizar_reserva,
            commands::reserva::cancelar_reserva,
            commands::reserva::eliminar_reserva,
            commands::renta::listar_rentas,
            commands::renta::obtener_renta,
            commands::renta::crear_renta,
            commands::renta::actualizar_renta,
            commands::renta::cerrar_renta,
            commands::renta::cambiar_auto_renta,
            commands::renta::cancelar_renta,
            commands::renta::eliminar_renta,
            commands::renta::registrar_pago_renta,
            commands::renta::registrar_inspeccion_renta,
            commands::renta::rentas_activas,
            commands::usuario::listar_usuarios,
            commands::usuario::crear_usuario,
            commands::usuario::actualizar_usuario,
            commands::usuario::eliminar_usuario,
            commands::usuario::forzar_cambio_password_usuario,
            commands::usuario::desbloquear_usuario,
            commands::pii::get_pii_status,
            commands::pii::probar_clave_pii,
            commands::pii::guardar_clave_pii,
            commands::pii::eliminar_clave_pii,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Crea el usuario admin por defecto si no hay usuarios (puerto de database_sa.init_db)
pub fn seed_admin(pool: &crate::core::db::Pool) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = pool.get()?;
    if crate::repositories::usuario::UsuarioRepository::contar(&mut conn)? == 0 {
        let hash = crate::core::security::hash_password("admin123")?;
        crate::repositories::usuario::UsuarioRepository::insertar(
            &mut conn,
            "admin",
            &hash,
            "Administrador Principal",
            "Administrador",
            true,
        )?;
        log::info!("Usuario 'admin' creado por defecto (password: admin123, debe cambiar)");
    }
    Ok(())
}
