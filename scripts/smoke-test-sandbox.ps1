# smoke-test-sandbox.ps1 - Smoke test del instalador de DinamoRent dentro de
# Windows Sandbox (entorno aislado). Se ejecuta automaticamente al arrancar la
# sesion del Sandbox via LogonCommand en el .wsb.
#
# IMPORTANTE: este archivo es ASCII puro a proposito - Windows PowerShell 5.1
# lee los .ps1 sin BOM como ANSI/CP1252 y los caracteres acentuados o guiones
# largos (UTF-8) rompen el parseo. No agregar acentos ni caracteres especiales.
#
# Que valida (el bug del release v1.0.0): en un equipo limpio la app se colgaba
# esperando una BD Firebird inexistente. Este script instala, arranca y
# verifica que la app crea %APPDATA%\com.corjar.dinamorent\ con config.ini +
# dinamo_rent_v3.fdb y que el proceso responde (no se cuelga).
#
# El log se escribe en la carpeta compartida con el host (C:\shared\out,
# mapeada en el .wsb a D:\dinamo_rent_tr\scripts\) para leerlo tras cerrar
# el Sandbox.

$ErrorActionPreference = 'Continue'
$log = 'C:\shared\out\smoke-result.txt'   # carpeta compartida con el host (scripts/)
$installer = 'C:\shared\bundle\nsis\DinamoRent_1.0.1_x64-setup.exe'

function Write-Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $msg
    Write-Host $line
    Add-Content -Path $log -Value $line -Encoding UTF8
}

Set-Content -Path $log -Value "=== Smoke test DinamoRent $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') ===" -Encoding UTF8

if (-not (Test-Path $installer)) {
    Write-Log "FALLO: no se encontro el instalador en $installer"
    exit 1
}
Write-Log "Instalador presente: $([math]::Round((Get-Item $installer).Length / 1MB, 1)) MB"

# 1) Instalacion silenciosa (NSIS /S)
Write-Log "Instalando silenciosamente..."
$proc = Start-Process -FilePath $installer -ArgumentList '/S' -PassThru -Wait
Write-Log "Instalador termino con codigo: $($proc.ExitCode)"

# Ruta real de instalacion NSIS (el exe se llama dinamo-rent.exe y va a
# %LOCALAPPDATA%\DinamoRent, no a Programs\... como los Templates viejos)
$exe = $null
$cands = @(
    "$env:LOCALAPPDATA\DinamoRent\dinamo-rent.exe",
    "$env:LOCALAPPDATA\DinamoRent\DinamoRent.exe",
    "$env:LOCALAPPDATA\Programs\DinamoRent\dinamo-rent.exe",
    "$env:LOCALAPPDATA\Programs\DinamoRent\DinamoRent.exe",
    "$env:ProgramFiles\DinamoRent\dinamo-rent.exe",
    "$env:ProgramFiles\DinamoRent\DinamoRent.exe"
)
foreach ($c in $cands) { if (Test-Path $c) { $exe = $c; break } }
if (-not $exe) {
    Write-Log "Buscando por wildcard *dinamo*.exe..."
    $found = Get-ChildItem $env:LOCALAPPDATA, $env:ProgramFiles -Recurse -Filter '*dinamo*.exe' -ErrorAction SilentlyContinue | Select-Object -First 3
    foreach ($f in $found) { Write-Log "  candidato: $($f.FullName)" }
    if ($found) { $exe = $found[0].FullName }
}
if (-not $exe) {
    Write-Log "FALLO: no se encontro el ejecutable instalado"
    exit 1
}
Write-Log "Ejecutable instalado: $exe"

# 2) Arranque
Write-Log "Arrancando la app..."
$app = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 12   # ventana para que cree la BD y migre

$app.Refresh()
if ($app.HasExited) {
    Write-Log "FALLO: la app salio sola con codigo $($app.ExitCode) - posible cuelgue/bloqueo al crear la BD"
    exit 1
}
Write-Log "Proceso vivo tras 12s (PID $($app.Id), sin cuelgue aparente)"

# 3) Verificacion de la BD y config
$data = "$env:APPDATA\com.corjar.dinamorent"
$fdb = Join-Path $data 'dinamo_rent_v3.fdb'
$ini = Join-Path $data 'config.ini'

if (-not (Test-Path $data)) { Write-Log "FALLO: no se creo $data" } else { Write-Log "OK: $data creado" }
if (-not (Test-Path $fdb))   { Write-Log "FALLO: no se creo la BD $fdb" } else {
    $size = (Get-Item $fdb).Length / 1MB
    Write-Log "OK: BD creada ($([math]::Round($size,1)) MB) - el fix de instalacion limpia funciona"
}
if (-not (Test-Path $ini))   { Write-Log "FALLO: no se genero $ini" } else { Write-Log "OK: config.ini generado" }

# 4) Cierre limpio
Write-Log "Cerrando la app (cierre de prueba)..."
try { $app.CloseMainWindow() | Out-Null } catch {}
Start-Sleep -Seconds 3
$app.Refresh()
if ($app.HasExited) { Write-Log "OK: la app cerro limpiamente con codigo $($app.ExitCode)" }
else { Write-Log "INFO: la app sigue abierta (la cerrara el Sandbox al terminar la sesion)" }

$veredicto = if ((Test-Path $fdb) -and (Test-Path $ini)) { 'OPERATIVO' } else { 'CON FALLOS' }
Write-Log "VEREDICTO: $veredicto"
Write-Log "=== Fin del smoke test ==="
