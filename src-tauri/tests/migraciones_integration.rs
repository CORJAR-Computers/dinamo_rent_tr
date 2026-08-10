//! migraciones_integration.rs — Pruebas del runner de migraciones contra una
//! COPIA temporal del .fdb de desarrollo (la BD real nunca se toca aquí).
//!
//! Cubre el bug que rompía el arranque de la app:
//!   - Una migración quedaba "a medias": `rentas.updated_at` creada pero
//!     `0007_triggers_updated_at.sql` sin registrar en schema_migrations
//!     (y sin los triggers). Al reintentar, el ALTER TABLE fallaba con
//!     "violation of PRIMARY or UNIQUE KEY constraint RDB$INDEX_15 on table
//!     RDB$RELATION_FIELDS" (columna duplicada).
//!   - La corrección: migraciones 0005-0009 idempotentes (EXECUTE BLOCK con
//!     guard contra el catálogo + RECREATE TRIGGER) y errores descriptivos en
//!     el runner (autocommit por sentencia) → el arranque se auto-repara y
//!     las ejecuciones sucesivas son no-op.
//!
//! Nota: el runner ejecuta cada sentencia en autocommit (una transacción por
//! migración rompe 0003/0004, que combinan DDL + UPDATE backfill: Firebird no
//! ve dentro de la misma transacción el DDL que acaba de ejecutar). La defensa
//! contra estados parciales es la IDEMPOTENCIA de TODAS las migraciones
//! (0001-0011): una instalación nueva que quedó a medias se auto-repara igual
//! que una BD existente. 0001 solo se ejecuta de verdad sobre BD vacía (en BDs
//! existentes se registra sin ejecutar vía `has_initial_schema`).

use std::path::PathBuf;
use std::sync::Arc;

use rsfbclient::{Execute, Queryable};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::db::{create_pool, Pool};
use dinamo_rent_lib::core::migrations::{has_initial_schema, run_migrations, split_sql_statements};

/// Borra el .fdb temporal al salir del scope (panic-safe).
struct LimpiarTemporal(PathBuf);
impl Drop for LimpiarTemporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Copia la BD de desarrollo a un archivo temporal (devuelve ruta + guard).
fn copia_bd_dev() -> (PathBuf, LimpiarTemporal) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("../data/dinamo_rent_v3.fdb");
    assert!(src.exists(), "BD de desarrollo no encontrada: {src:?}");
    let tmp = std::env::temp_dir().join(format!(
        "dinamo_rent_migraciones_{}.fdb",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::copy(&src, &tmp).expect("copiar .fdb a temporal");
    (tmp.clone(), LimpiarTemporal(tmp))
}

/// Config de dev pero apuntando a la copia temporal.
fn config_con_db(path: &PathBuf) -> Arc<AppConfig> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let mut cfg = AppConfig::load(&data_dir, &resource_dir, &manifest);
    cfg.db_path = path.clone();
    Arc::new(cfg)
}

/// Crea una BD Firebird vacía temporal con el mismo builder embedded del pool
/// (camino de instalación nueva). Devuelve el pool y el guard de limpieza.
fn crear_bd_fresh() -> (Pool, LimpiarTemporal) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let base = AppConfig::load(&data_dir, &resource_dir, &manifest);

    let tmp = std::env::temp_dir().join(format!(
        "dinamo_rent_fresh_{}.fdb",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let limpieza = LimpiarTemporal(tmp.clone());

    let mut builder = rsfbclient::builder_native()
        .with_dyn_load(base.fbclient_path.to_string_lossy().to_string())
        .with_embedded();
    builder.db_name(tmp.to_string_lossy().to_string());
    builder.user(base.db_user.clone());
    builder
        .create_database()
        .expect("crear BD vacía con rsfbclient");

    let mut cfg = base;
    cfg.db_path = tmp;
    let pool = create_pool(&Arc::new(cfg)).expect("pool embedded");
    (pool, limpieza)
}

fn versiones_aplicadas(pool: &Pool) -> Vec<String> {
    let mut conn = pool.get().expect("conn");
    conn.query("SELECT version FROM schema_migrations", ())
        .map(|rows: Vec<(String,)>| rows.into_iter().map(|r| r.0).collect())
        .unwrap_or_default()
}

fn existe_columna(pool: &Pool, tabla: &str, columna: &str) -> bool {
    let mut conn = pool.get().expect("conn");
    let r: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM RDB$RELATION_FIELDS \
             WHERE RDB$RELATION_NAME = ? AND RDB$FIELD_NAME = ?",
            (tabla.to_string(), columna.to_string()),
        )
        .expect("consulta catálogo columnas");
    r.map(|(c,)| c > 0).unwrap_or(false)
}

fn existe_objeto(pool: &Pool, sql: &str, nombre: &str) -> bool {
    let mut conn = pool.get().expect("conn");
    let r: Option<(i64,)> = conn
        .query_first(sql, (nombre.to_string(),))
        .expect("consulta catálogo");
    r.map(|(c,)| c > 0).unwrap_or(false)
}

/// Nota: las columnas de metadatos de Firebird (RDB$TRIGGER_NAME, etc.) se
/// comparan con semántica CHAR (espacios al final), así que los patrones LIKE
/// que terminan en literal no matchean. Se usa TRIM() para comparar el nombre
/// real.
fn contar_triggers_updated_at(pool: &Pool) -> i64 {
    let mut conn = pool.get().expect("conn");
    let r: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM RDB$TRIGGERS \
             WHERE RDB$SYSTEM_FLAG = 0 AND TRIM(RDB$TRIGGER_NAME) LIKE 'TRG_%UPDATED_AT'",
            (),
        )
        .expect("contar triggers");
    r.map(|(c,)| c).unwrap_or(0)
}

fn contar_chk(pool: &Pool) -> i64 {
    let mut conn = pool.get().expect("conn");
    let r: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM RDB$RELATION_CONSTRAINTS \
             WHERE TRIM(RDB$CONSTRAINT_NAME) LIKE 'CHK_%'",
            (),
        )
        .expect("contar constraints");
    r.map(|(c,)| c).unwrap_or(0)
}

/// Recrea (de forma determinista) el estado roto original en la copia: borra
/// los registros 0007-0009 de schema_migrations y elimina los objetos que
/// crean esas migraciones, dejando rentas.updated_at (la "aplicación parcial"
/// histórica que causaba el fallo de arranque).
///
/// Nota: NO elimina IX_INSPECCIONES_ID_RENTA — con el 0009 actual (que ya no
/// crea ese índice) no hay objeto que recrear, y dejarlo presente permite que
/// la consolidación 0011 (bloque C3) ejercite su DROP sobre la copia de dev.
fn forzar_estado_parcial(pool: &Pool) {
    let mut conn = pool.get().expect("conn");
    conn.execute(
        "DELETE FROM schema_migrations \
         WHERE version LIKE '0007%' OR version LIKE '0008%' OR version LIKE '0009%'",
        (),
    )
    .expect("borrar versiones 0007-0009");

    for trig in [
        "TRG_USUARIOS_UPDATED_AT",
        "TRG_AUTOS_UPDATED_AT",
        "TRG_CLIENTES_UPDATED_AT",
        "TRG_RESERVAS_UPDATED_AT",
        "TRG_RENTAS_UPDATED_AT",
        "TRG_MANTENIMIENTO_VEHICULOS_UPDATED_AT",
        "TRG_GASTOS_UPDATED_AT",
        "TRG_COMPARENDOS_UPDATED_AT",
        "TRG_PAGOS_UPDATED_AT",
    ] {
        let sql = "SELECT COUNT(*) FROM RDB$TRIGGERS \
                   WHERE RDB$SYSTEM_FLAG = 0 AND RDB$TRIGGER_NAME = ?";
        if existe_objeto(pool, sql, trig) {
            conn.execute(&format!("DROP TRIGGER {trig}"), ())
                .expect("drop trigger");
        }
    }
    for (tabla, chk) in [
        ("RENTAS", "CHK_RENTAS_ESTADO"),
        ("AUTOS", "CHK_AUTOS_ESTADO"),
        ("CLIENTES", "CHK_CLIENTES_ESTADO"),
        ("RESERVAS", "CHK_RESERVAS_ESTADO"),
        ("COMPARENDOS", "CHK_COMPARENDOS_ESTADO"),
    ] {
        let sql =
            "SELECT COUNT(*) FROM RDB$RELATION_CONSTRAINTS WHERE RDB$CONSTRAINT_NAME = ?";
        if existe_objeto(pool, sql, chk) {
            conn.execute(&format!("ALTER TABLE {tabla} DROP CONSTRAINT {chk}"), ())
                .expect("drop constraint");
        }
    }
}

#[test]
#[serial]
fn migraciones_auto_reparan_estado_parcial_y_son_idempotentes() {
    let (tmp, _limpieza) = copia_bd_dev();
    let cfg = config_con_db(&tmp);
    let pool = create_pool(&cfg).expect("pool embedded");
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");

    // Reproducir el escenario del bug: estado parcial de 0007-0009.
    forzar_estado_parcial(&pool);

    // Precondición del bug: rentas.updated_at YA existe pero 0007 NO está
    // registrada → el runner antiguo habría abortado aquí.
    assert!(
        existe_columna(&pool, "RENTAS", "UPDATED_AT"),
        "precondición: rentas.updated_at debe existir (estado parcial)"
    );
    assert!(
        !versiones_aplicadas(&pool)
            .iter()
            .any(|v| v.starts_with("0007")),
        "precondición: 0007 no debe estar registrada"
    );

    // 1ª ejecución: debe AUTO-REPARAR sin error.
    run_migrations(&pool, &migrations_dir)
        .expect("la migración debe auto-reparar la BD parcial");

    // Las 11 migraciones quedan registradas.
    let aplicadas = versiones_aplicadas(&pool);
    for v in [
        "0001_initial_schema.sql",
        "0002_indices_optimizacion.sql",
        "0003_no_contrato.sql",
        "0004_no_contrato_anual.sql",
        "0005_tema_usuario.sql",
        "0006_soft_deletes.sql",
        "0007_triggers_updated_at.sql",
        "0008_check_constraints.sql",
        "0009_indices.sql",
        "0010_dedup_indices.sql",
        "0011_consolidar_indices.sql",
    ] {
        assert!(
            aplicadas.contains(&v.to_string()),
            "falta registrar {v}; registradas: {aplicadas:?}"
        );
    }

    // Objetos de 0007-0009 presentes (y columnas previas intactas).
    assert_eq!(contar_triggers_updated_at(&pool), 9, "9 triggers updated_at");
    assert!(existe_columna(&pool, "RENTAS", "UPDATED_AT"));
    assert!(existe_columna(&pool, "RENTAS", "DELETED_AT"));
    assert!(existe_columna(&pool, "USUARIOS", "TEMA"));
    assert_eq!(contar_chk(&pool), 5, "5 CHECK constraints");
    // La consolidación 0011 (bloque C3) elimina IX_INSPECCIONES_ID_RENTA: la
    // copia de dev lo tenía (forzar_estado_parcial ya no lo borra) y la FK
    // RDB$FOREIGN42 sigue cubriendo inspecciones.id_renta.
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_INSPECCIONES_ID_RENTA"
    ));

    // Consolidación 0011 sobre la copia de dev:
    //  - 0010 ya deduplicó mantenimiento(placa) en la copia (quedó solo
    //    IDX_MANTENIMIENTO_PLACA, que ahora 0011 elimina por redundante con
    //    la FK y con IX_MANTENIMIENTO_PLACA_FECHA).
    //  - rentas(placa): la copia solo tenía el IX_ de 0001; 0011 crea el
    //    IDX_RENTAS_PLACA canónico (B1) y elimina el IX_ (B2).
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_MANTENIMIENTO_VEHICULOS_PLACA"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_MANTENIMIENTO_PLACA"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_PLACA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_RENTAS_PLACA"
    ));
    // El resto de índices legacy/redundantes que 0011 elimina de la copia de
    // dev (los cubren compuestos o las FKs):
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_ESTADO"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_ID_CLIENTE"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_GASTOS_PLACA"
    ));
    // Los índices de COBERTURA deben sobrevivir a la consolidación (los guards
    // de 0011 nunca deben tocar los compuestos canónicos):
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_RENTAS_ESTADO"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_GASTOS_PLACA_FECHA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_MANTENIMIENTO_PLACA_FECHA"
    ));

    // 2ª ejecución: idempotente, sin error y sin añadir versiones.
    run_migrations(&pool, &migrations_dir).expect("segunda ejecución debe ser no-op");
    assert_eq!(
        versiones_aplicadas(&pool),
        aplicadas,
        "la segunda ejecución no debe añadir versiones"
    );
}

#[test]
#[serial]
fn migraciones_sobre_bd_ya_migrada_son_no_op() {
    let (tmp, _limpieza) = copia_bd_dev();
    let cfg = config_con_db(&tmp);
    let pool = create_pool(&cfg).expect("pool embedded");
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");

    // Copia ya migrada (o migrada por la 1ª ejecución): la 2ª no debe fallar
    // ni duplicar objetos.
    run_migrations(&pool, &migrations_dir).expect("run 1");
    let antes = versiones_aplicadas(&pool);
    run_migrations(&pool, &migrations_dir).expect("run 2");
    assert_eq!(versiones_aplicadas(&pool), antes);
    assert_eq!(contar_triggers_updated_at(&pool), 9);
    assert_eq!(contar_chk(&pool), 5);
}

/// Borra el directorio temporal al salir (panic-safe).
struct LimpiarDir(PathBuf);
impl Drop for LimpiarDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Una migración con SQL inválido debe ABORTAR sin registrar su versión y con
/// un error que identifique la migración y la sentencia (recuperabilidad: el
/// siguiente arranque la reintenta).
///
/// Nota: el runner ejecuta cada sentencia en autocommit (una transacción por
/// migración rompería 0003/0004), así que las sentencias anteriores a la que
/// falla PERSISTEN. Es exactamente el estado parcial que las migraciones
/// 0005-0009 idempotentes auto-reparan — aquí lo verificamos con una migración
/// rota: la versión NO queda registrada y la segunda ejecución (tras arreglar
/// el SQL) completa el trabajo.
#[test]
#[serial]
fn migracion_que_falla_no_se_registra_y_es_reintentable() {
    let (tmp, _limpieza) = copia_bd_dev();
    let cfg = config_con_db(&tmp);
    let pool = create_pool(&cfg).expect("pool embedded");

    // Directorio de migraciones temporal con UNA migración rota: la primera
    // sentencia es válida y la segunda es SQL inválido.
    let dir = std::env::temp_dir().join(format!(
        "dinamo_rent_mig_dir_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("crear dir migraciones temporal");
    let _limpieza_dir = LimpiarDir(dir.clone());
    std::fs::write(
        dir.join("0000_bad.sql"),
        "CREATE TABLE mig_parcial (id INTEGER);\n\
         ESTO_NO_ES_SQL_INVALIDO;",
    )
    .expect("escribir migración rota");

    let err = run_migrations(&pool, &dir).expect_err("la migración rota debe fallar");
    let msg = format!("{err}");
    assert!(
        msg.contains("0000_bad.sql"),
        "el error debe identificar la migración: {msg}"
    );
    assert!(
        msg.contains("2/2"),
        "el error debe identificar la sentencia 2 de 2: {msg}"
    );

    let mut conn = pool.get().expect("conn");
    // La versión no debe quedar registrada.
    let n: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = '0000_bad.sql'",
            (),
        )
        .expect("contar versión");
    assert_eq!(
        n.map(|(c,)| c).unwrap_or(0),
        0,
        "no debe registrarse la versión de una migración fallida"
    );

    // Recuperabilidad: corregimos la migración rota (nota: como el runner
    // ejecuta en autocommit, la primera sentencia ya aplicó en el intento
    // anterior; la versión corregida no debe chocar con lo ya creado — igual
    // que las migraciones 0005-0009, que son idempotentes). Al reintentar se
    // completa y la versión queda registrada.
    std::fs::write(
        dir.join("0000_bad.sql"),
        "CREATE TABLE mig_parcial_nueva (id INTEGER);\n\
         CREATE TABLE mig_parcial2 (id INTEGER);",
    )
    .expect("corregir migración");
    run_migrations(&pool, &dir).expect("reintento debe completar la migración");
    let n: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = '0000_bad.sql'",
            (),
        )
        .expect("contar versión tras reintento");
    assert_eq!(
        n.map(|(c,)| c).unwrap_or(0),
        1,
        "tras el reintento la versión debe quedar registrada"
    );
}

/// Las 11 migraciones deben aplicarse desde cero sobre una BD NUEVA y vacía
/// (camino de instalación en máquinas nuevas: 0001 crea todo el esquema).
#[test]
#[serial]
fn migraciones_aplican_en_bd_nueva_desde_cero() {
    let (pool, _limpieza) = crear_bd_fresh();
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");

    run_migrations(&pool, &migrations_dir)
        .expect("las 11 migraciones deben aplicar sobre una BD vacía");

    let aplicadas = versiones_aplicadas(&pool);
    for v in [
        "0001_initial_schema.sql",
        "0005_tema_usuario.sql",
        "0007_triggers_updated_at.sql",
        "0008_check_constraints.sql",
        "0009_indices.sql",
        "0010_dedup_indices.sql",
        "0011_consolidar_indices.sql",
    ] {
        assert!(
            aplicadas.contains(&v.to_string()),
            "falta registrar {v} en BD nueva; registradas: {aplicadas:?}"
        );
    }
    assert!(existe_columna(&pool, "RENTAS", "UPDATED_AT"));
    assert!(existe_columna(&pool, "RENTAS", "NO_CONTRATO"));
    assert!(existe_columna(&pool, "RENTAS", "ANIO_CONTRATO"));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$GENERATORS WHERE RDB$GENERATOR_NAME = ?",
        "GEN_RENTA_NO_CONTRATO"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO_ANIO"
    ));
    // 0004 dropea el índice único de 0003 y lo reemplaza por el anual.
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO"
    ));
    assert!(existe_columna(&pool, "USUARIOS", "TEMA"));
    assert_eq!(contar_triggers_updated_at(&pool), 9);
    assert_eq!(contar_chk(&pool), 5);
    // Consolidación en BD nueva: 0001/0002/0009 ya NO crean los índices
    // redundantes (los cubren los compuestos y las FKs), así que 0010/0011
    // son no-op. Esquema canónico esperado:
    //  - rentas(placa): solo IDX_RENTAS_PLACA (0002).
    //  - mantenimiento(placa): solo IX_MANTENIMIENTO_PLACA_FECHA (0001).
    //  - inspecciones(id_renta): solo el índice de la FK (0009 no crea manual).
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_INSPECCIONES_ID_RENTA"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_PLACA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_RENTAS_PLACA"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_MANTENIMIENTO_VEHICULOS_PLACA"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_MANTENIMIENTO_PLACA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_MANTENIMIENTO_PLACA_FECHA"
    ));
    // Los demás legacy eliminados de 0001 no deben existir en instalación
    // nueva; los compuestos canónicos sí.
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_ESTADO"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_ID_CLIENTE"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_GASTOS_PLACA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_GASTOS_PLACA_FECHA"
    ));

    // Idempotente también en el camino de BD nueva.
    run_migrations(&pool, &migrations_dir).expect("segunda ejecución en BD nueva");
}

/// `has_initial_schema` debe exigir las 4 tablas núcleo (USUARIOS, AUTOS,
/// CLIENTES, RENTAS) MÁS `pagos` (la última tabla de 0001): sobre una BD vacía
/// o con esquema parcial devuelve false, de modo que el runner NO registra 0001
/// como aplicada a la ligera (registrarla sobre un esquema incompleto dejaría
/// la instalación rota sin ejecutar nunca el resto de 0001).
#[test]
#[serial]
fn has_initial_schema_exige_las_4_tablas_nucleo() {
    let (pool, _limpieza) = crear_bd_fresh();

    // BD vacía: sin esquema inicial.
    assert!(
        !has_initial_schema(&pool),
        "BD vacía no debe reportar esquema inicial"
    );

    let mut conn = pool.get().expect("conn");

    // Solo USUARIOS (esquema parcial): no basta.
    conn.execute("CREATE TABLE usuarios (id INTEGER)", ())
        .expect("crear usuarios");
    assert!(
        !has_initial_schema(&pool),
        "1/5 tablas no debe reportar esquema inicial"
    );

    // AUTOS + CLIENTES + RENTAS: las 4 núcleo presentes, pero 0001 NO terminó
    // (falta PAGOS) → sigue sin ser esquema inicial (crash tras RENTAS).
    conn.execute("CREATE TABLE autos (placa VARCHAR(20) PRIMARY KEY)", ())
        .expect("crear autos");
    conn.execute("CREATE TABLE clientes (id INTEGER)", ())
        .expect("crear clientes");
    conn.execute("CREATE TABLE rentas (id INTEGER)", ())
        .expect("crear rentas");
    assert!(
        !has_initial_schema(&pool),
        "4 tablas núcleo sin PAGOS: 0001 quedó a medias, no es esquema inicial"
    );

    // PAGOS presente → esquema inicial completo.
    conn.execute("CREATE TABLE pagos (id INTEGER)", ())
        .expect("crear pagos");
    assert!(
        has_initial_schema(&pool),
        "con PAGOS: esquema inicial completo"
    );

    // La BD de desarrollo (copia) siempre debe reportar esquema completo.
    let (tmp, _limpieza2) = copia_bd_dev();
    let cfg2 = config_con_db(&tmp);
    let pool2 = create_pool(&cfg2).expect("pool embedded");
    assert!(
        has_initial_schema(&pool2),
        "la BD de desarrollo tiene el esquema inicial completo"
    );
}

/// Simula una instalación nueva que se interrumpió a mitad de 0001 (crash
/// antes de registrar versiones): los guards de 0001 omiten las 6 sentencias
/// ya aplicadas y crean el resto. El runner debe completar TODO, registrar las
/// 11 versiones y quedar idempotente.
#[test]
#[serial]
fn instalacion_nueva_a_medias_en_0001_se_auto_repara() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let migrations_dir = manifest.join("migrations");
    let leer = |f: &str| std::fs::read_to_string(migrations_dir.join(f)).unwrap();

    let (pool, _limpieza) = crear_bd_fresh();
    let stmts_0001 = split_sql_statements(&leer("0001_initial_schema.sql"));
    let mut conn = pool.get().expect("conn");
    for stmt in &stmts_0001[..6] {
        conn.execute(stmt, ())
            .unwrap_or_else(|e| panic!("aplicar parcial 0001: {e}"));
    }
    // Estado realmente parcial: ni siquiera RENTAS existe aún (0001 iba por la
    // sección de auditoria) y 0002 no puede haberse aplicado.
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$RELATIONS WHERE RDB$RELATION_NAME = ?",
        "RENTAS"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_RENTAS_PLACA"
    ));

    run_migrations(&pool, &migrations_dir)
        .expect("el runner debe auto-reparar una instalación nueva a medias");

    assert_eq!(versiones_aplicadas(&pool).len(), 11, "todas las versiones");
    // 0001 completo: la última tabla y su índice; 0002 completo.
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$RELATIONS WHERE RDB$RELATION_NAME = ?",
        "PAGOS"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_PAGOS_RENTA_FECHA"
    ));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IDX_RENTAS_PLACA"
    ));
    // 0001 ya no crea IX_RENTAS_PLACA (solo IDX_RENTAS_PLACA de 0002).
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_PLACA"
    ));
    // 0003/0004 completos.
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$GENERATORS WHERE RDB$GENERATOR_NAME = ?",
        "GEN_RENTA_NO_CONTRATO"
    ));
    assert!(existe_columna(&pool, "RENTAS", "ANIO_CONTRATO"));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO_ANIO"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO"
    ));
    // 0005-0009 presentes; 0009 ya no crea el índice manual de inspecciones.
    assert!(existe_columna(&pool, "USUARIOS", "TEMA"));
    assert_eq!(contar_triggers_updated_at(&pool), 9);
    assert_eq!(contar_chk(&pool), 5);
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_INSPECCIONES_ID_RENTA"
    ));
    run_migrations(&pool, &migrations_dir).expect("segunda ejecución no-op");
    assert_eq!(versiones_aplicadas(&pool).len(), 11);
}

/// Simula una instalación nueva con 0001+0002 completos y un crash a mitad de
/// 0003 y 0004 (backfills y DROP/CREATE de índices quedan pendientes). El
/// runner debe completarlos con los guards, registrar las 11 versiones y
/// quedar idempotente.
#[test]
#[serial]
fn instalacion_nueva_a_medias_en_0003_0004_se_auto_repara() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let migrations_dir = manifest.join("migrations");
    let leer = |f: &str| std::fs::read_to_string(migrations_dir.join(f)).unwrap();

    let (pool, _limpieza) = crear_bd_fresh();
    let mut conn = pool.get().expect("conn");
    for file in ["0001_initial_schema.sql", "0002_indices_optimizacion.sql"] {
        for stmt in split_sql_statements(&leer(file)) {
            conn.execute(&stmt, ())
                .unwrap_or_else(|e| panic!("aplicar {file}: {e}"));
        }
    }
    // 0003 a medias (sin backfill ni índice único) y 0004 a medias (solo ALTER).
    let stmts_0003 = split_sql_statements(&leer("0003_no_contrato.sql"));
    for stmt in &stmts_0003[..2] {
        conn.execute(stmt, ())
            .unwrap_or_else(|e| panic!("aplicar parcial 0003: {e}"));
    }
    let stmts_0004 = split_sql_statements(&leer("0004_no_contrato_anual.sql"));
    conn
        .execute(&stmts_0004[0], ())
        .unwrap_or_else(|e| panic!("aplicar parcial 0004: {e}"));

    assert!(existe_columna(&pool, "RENTAS", "NO_CONTRATO"));
    assert!(
        !existe_objeto(
            &pool,
            "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
            "IX_RENTAS_NO_CONTRATO"
        ),
        "precondición: 0003 quedó a medias (índice único sin crear)"
    );

    run_migrations(&pool, &migrations_dir)
        .expect("auto-reparar instalación a medias en 0003/0004");

    assert_eq!(versiones_aplicadas(&pool).len(), 11);
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$GENERATORS WHERE RDB$GENERATOR_NAME = ?",
        "GEN_RENTA_NO_CONTRATO"
    ));
    assert!(existe_columna(&pool, "RENTAS", "ANIO_CONTRATO"));
    assert!(existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO_ANIO"
    ));
    assert!(!existe_objeto(
        &pool,
        "SELECT COUNT(*) FROM RDB$INDICES WHERE RDB$INDEX_NAME = ?",
        "IX_RENTAS_NO_CONTRATO"
    ));
    run_migrations(&pool, &migrations_dir).expect("segunda ejecución no-op");
    assert_eq!(versiones_aplicadas(&pool).len(), 11);
}
