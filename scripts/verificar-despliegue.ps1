# verificar-despliegue.ps1 - Verificacion post-instalacion de DinamoRent v1.0.21
#
# Corre en el equipo objetivo como usuario normal:
#   powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1
#
# IMPORTANTE: ASCII puro a proposito (Windows PowerShell 5.1 lee los .ps1 sin BOM
# como ANSI/CP1252 y los acentos/guiones largos UTF-8 rompen el parseo).
#
# Comprueba: exe instalado (v1.0.21), %APPDATA%\com.corjar.dinamorent (config.ini
# + dinamo_rent_v3.fdb) y que la app arranca y queda viva 10 s (el bug del v1.0.0
# era justamente morirse antes del Login). Ver DEPLOYMENT_CLIENTES.md.

<#
.SYNOPSIS
Verificacion post-instalacion de DinamoRent (exe, datos y arranque).

.DESCRIPTION
Comprueba en el equipo objetivo: exe instalado con la version esperada,
%APPDATA%\com.corjar.dinamorent con config.ini + dinamo_rent_v3.fdb, y que
la app arranca y queda viva 10 s. Termina con "VEREDICTO: OK" y exit 0 si
todo pasa, o "VEREDICTO: FALLOS" y exit 1 si alguna comprobacion falla.

.PARAMETER DryRun
No toca la maquina real: ejecuta los chequeos y el veredicto reales contra
un ambiente simulado en %TEMP% (exe, config.ini y BD falsos). Se usa en el
CI (paso "Verificador de despliegue (-DryRun)" de ci.yml) para validar el
flujo del script sin instalar nada.

.PARAMETER SimularFallo
Solo tiene sentido con -DryRun. Fuerza el camino de FALLOS (version vieja,
app muerta, sin config.ini ni BD) para probar que el veredicto termina con
"VEREDICTO: FALLOS" y exit 1.

.EXAMPLE
powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1

Verificacion real sobre una instalacion hecha (equipo del cliente).

.EXAMPLE
powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1 -DryRun

Caso OK simulado: debe terminar con "VEREDICTO: OK" y exit 0.

.EXAMPLE
powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1 -DryRun -SimularFallo

Camino de FALLOS simulado: debe terminar con "VEREDICTO: FALLOS" y exit 1.
#>

param(
    [switch]$DryRun,
    [switch]$SimularFallo
)

# -DryRun: valida el flujo del script (chequeos, salida y veredicto) contra
# un ambiente simulado en TEMP, sin tocar la maquina real. -SimularFallo
# solo tiene sentido con -DryRun: fuerza el camino de FALLOS (version vieja,
# app muerta, sin config.ini ni BD) para probar el veredicto.

$ErrorActionPreference = 'Continue'
$failed = @()
$ok = @()

function Check([string]$name, [bool]$cond, [string]$detail = '') {
    if ($cond) {
        Write-Host ("  [OK] " + $name + $(if ($detail) { " - " + $detail } else { "" }))
        $script:ok += $name
    } else {
        Write-Host ("  [FALLO] " + $name + $(if ($detail) { " - " + $detail } else { "" }))
        $script:failed += $name
    }
}

Write-Host "=== Verificacion de despliegue DinamoRent $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') ==="

# --- Modo DryRun: ambiente simulado en TEMP ---
$dryBase = $null
if ($DryRun) {
    Write-Host "  [dry-run] ambiente simulado en TEMP (no toca la maquina real)"
    $dryBase = Join-Path $env:TEMP ("dinamorent-dryrun-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path (Join-Path $dryBase 'DinamoRent') | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $dryBase 'com.corjar.dinamorent') | Out-Null
    if (-not $SimularFallo) {
        # Caso OK: config.ini + BD simulada (~5 MB)
        New-Item -ItemType File -Force -Path (Join-Path $dryBase 'com.corjar.dinamorent\config.ini') | Out-Null
        $fdbBytes = New-Object byte[] (5 * 1024 * 1024)
        [IO.File]::WriteAllBytes((Join-Path $dryBase 'com.corjar.dinamorent\dinamo_rent_v3.fdb'), $fdbBytes)
    }
}

# 1) Ejecutable instalado y version
$exe = $null
if ($DryRun) {
    $exe = Join-Path $dryBase 'DinamoRent\dinamo-rent.exe'
    New-Item -ItemType File -Force -Path $exe | Out-Null
} else {
    $cands = @(
        "$env:LOCALAPPDATA\DinamoRent\dinamo-rent.exe",
        "$env:LOCALAPPDATA\Programs\DinamoRent\dinamo-rent.exe",
        "$env:ProgramFiles\DinamoRent\dinamo-rent.exe"
    )
    foreach ($c in $cands) { if (Test-Path $c) { $exe = $c; break } }
    if (-not $exe) {
        $exe = Get-ChildItem $env:LOCALAPPDATA, $env:ProgramFiles -Recurse -Filter 'dinamo-rent.exe' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
    }
}
if ($exe) {
    if ($DryRun) {
        $ver = if ($SimularFallo) { '1.0.15.0' } else { '1.0.21.0' }
    } else {
        $ver = (Get-Item $exe).VersionInfo.ProductVersion
    }
    Check "Ejecutable instalado" $true $exe
    Check "Version 1.0.21" ($ver -like '1.0.21*') "ProductVersion=$ver"
} else {
    Check "Ejecutable instalado" $false 'no se encontro dinamo-rent.exe'
}

# 2) Arranque: proceso vivo tras 10 s (PRIMERO: el primer arranque es el que
#    crea %APPDATA%\com.corjar.dinamorent\ con config.ini + BD. Si los datos se
#    comprobaran antes, una instalacion recien hecha fallaria falsamente.)
$app = $null
if ($exe) {
    if ($DryRun) {
        Write-Host "  [dry-run] simulando arranque de 10 s..."
        Start-Sleep -Milliseconds 400
        if ($SimularFallo) {
            Check "App viva tras 10 s (sin cuelgue)" $false 'simulado: salio sola'
        } else {
            Check "App viva tras 10 s (sin cuelgue)" $true 'simulado: PID fake'
        }
    } else {
        Write-Host "  Arrancando la app (10 s)..."
        $app = Start-Process -FilePath $exe -PassThru
        Start-Sleep -Seconds 10
        $app.Refresh()
        if (-not $app.HasExited) {
            Check "App viva tras 10 s (sin cuelgue)" $true "PID $($app.Id)"
        } else {
            Check "App viva tras 10 s (sin cuelgue)" $false "salio sola con codigo $($app.ExitCode)"
        }
    }
} else {
    Write-Host "  (sin exe: no se puede probar el arranque)"
}

# 3) Carpeta de datos (debe existir tras el primer arranque)
$data = if ($DryRun) { Join-Path $dryBase 'com.corjar.dinamorent' } else { "$env:APPDATA\com.corjar.dinamorent" }
$fdb = Join-Path $data 'dinamo_rent_v3.fdb'
$ini = Join-Path $data 'config.ini'
Check "Carpeta de datos creada" (Test-Path $data) $data
Check "config.ini generado" (Test-Path $ini)
if (Test-Path $fdb) {
    $sz = (Get-Item $fdb).Length
    Check "BD dinamo_rent_v3.fdb existe" ($sz -gt 0) ("$([math]::Round($sz/1MB,1)) MB")
} else {
    Check "BD dinamo_rent_v3.fdb existe" $false 'no se creo (bug v1.0.0: cuelgue aqui)'
}

# 4) Cierre de prueba
if ($app) {
    try { $app.CloseMainWindow() | Out-Null } catch {}
    Start-Sleep -Seconds 2
}

# 5) Veredicto
if ($DryRun -and $dryBase) { Remove-Item -Recurse -Force $dryBase -ErrorAction SilentlyContinue }
Write-Host ""
if ($failed.Count -eq 0) {
    Write-Host "=== VEREDICTO: OK ==="
    Write-Host ("  " + $ok.Count + " comprobaciones correctas.")
    exit 0
} else {
    Write-Host "=== VEREDICTO: FALLOS ==="
    Write-Host ("  Fallidas: " + ($failed -join ', '))
    Write-Host "  Ver DEPLOYMENT_CLIENTES.md seccion 3 (si algo falla) y 4 (rollback)."
    exit 1
}
