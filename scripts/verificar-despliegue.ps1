# verificar-despliegue.ps1 - Verificacion post-instalacion de DinamoRent v1.0.9
#
# Corre en el equipo objetivo como usuario normal:
#   powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1
#
# IMPORTANTE: ASCII puro a proposito (Windows PowerShell 5.1 lee los .ps1 sin BOM
# como ANSI/CP1252 y los acentos/guiones largos UTF-8 rompen el parseo).
#
# Comprueba: exe instalado (v1.0.9), %APPDATA%\com.corjar.dinamorent (config.ini
# + dinamo_rent_v3.fdb) y que la app arranca y queda viva 10 s (el bug del v1.0.0
# era justamente morirse antes del Login). Ver DEPLOYMENT_CLIENTES.md.

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

# 1) Ejecutable instalado y version
$exe = $null
$cands = @(
    "$env:LOCALAPPDATA\DinamoRent\dinamo-rent.exe",
    "$env:LOCALAPPDATA\Programs\DinamoRent\dinamo-rent.exe",
    "$env:ProgramFiles\DinamoRent\dinamo-rent.exe"
)
foreach ($c in $cands) { if (Test-Path $c) { $exe = $c; break } }
if (-not $exe) {
    $exe = Get-ChildItem $env:LOCALAPPDATA, $env:ProgramFiles -Recurse -Filter 'dinamo-rent.exe' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
}
if ($exe) {
    $ver = (Get-Item $exe).VersionInfo.ProductVersion
    Check "Ejecutable instalado" $true $exe
    Check "Version 1.0.10" ($ver -like '1.0.10*') "ProductVersion=$ver"
} else {
    Check "Ejecutable instalado" $false 'no se encontro dinamo-rent.exe'
}

# 2) Arranque: proceso vivo tras 10 s (PRIMERO: el primer arranque es el que
#    crea %APPDATA%\com.corjar.dinamorent\ con config.ini + BD. Si los datos se
#    comprobaran antes, una instalacion recien hecha fallaria falsamente.)
$app = $null
if ($exe) {
    Write-Host "  Arrancando la app (10 s)..."
    $app = Start-Process -FilePath $exe -PassThru
    Start-Sleep -Seconds 10
    $app.Refresh()
    if (-not $app.HasExited) {
        Check "App viva tras 10 s (sin cuelgue)" $true "PID $($app.Id)"
    } else {
        Check "App viva tras 10 s (sin cuelgue)" $false "salio sola con codigo $($app.ExitCode)"
    }
} else {
    Write-Host "  (sin exe: no se puede probar el arranque)"
}

# 3) Carpeta de datos (debe existir tras el primer arranque)
$data = "$env:APPDATA\com.corjar.dinamorent"
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
