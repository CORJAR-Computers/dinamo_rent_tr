# verificar-despliegue-sandbox.ps1 - Instala la v1.0.1 dentro de Windows
# Sandbox y ejecuta scripts\verificar-despliegue.ps1 de punta a punta para
# validar el script de verificacion contra una instalacion real y limpia.
#
# Se lanza automaticamente via LogonCommand en dinamorent-sandbox-verificar.wsb.
# El resultado queda en C:\shared\out\verificar-result.txt (scripts/ del host).
#
# IMPORTANTE: ASCII puro a proposito - Windows PowerShell 5.1 lee los .ps1 sin
# BOM como ANSI/CP1252 y los caracteres acentuados (UTF-8) rompen el parseo.
# No agregar acentos ni caracteres especiales.

$ErrorActionPreference = 'Continue'
$log = 'C:\shared\out\verificar-result.txt'
Set-Content -Path $log -Value "=== Validacion de verificar-despliegue.ps1 $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') ===" -Encoding UTF8

function Write-Log([string]$msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $msg
    Write-Host $line
    Add-Content -Path $log -Value $line -Encoding UTF8
}

$installer = 'C:\shared\bundle\nsis\DinamoRent_1.0.1_x64-setup.exe'
if (-not (Test-Path $installer)) {
    Write-Log "FALLO: no se encontro el instalador en $installer"
    exit 1
}
Write-Log "Instalador presente: $([math]::Round((Get-Item $installer).Length / 1MB, 1)) MB"

# 1) Instalacion silenciosa de la v1.0.1
Write-Log "Instalando la v1.0.1 (NSIS /S)..."
$proc = Start-Process -FilePath $installer -ArgumentList '/S' -PassThru -Wait
Write-Log "Instalador termino con codigo: $($proc.ExitCode)"

# 2) Ejecutar el script de verificacion en un proceso hijo (aisla su exit)
$verifier = 'C:\shared\out\verificar-despliegue.ps1'
if (-not (Test-Path $verifier)) {
    Write-Log "FALLO: no se encontro $verifier"
    exit 1
}
Write-Log "=== Ejecutando verificar-despliegue.ps1 ==="
$output = & powershell -NoProfile -ExecutionPolicy Bypass -File $verifier 2>&1
$code = $LASTEXITCODE
$output | ForEach-Object { Add-Content -Path $log -Value $_ -Encoding UTF8 }
Write-Log "verificar-despliegue.ps1 termino con codigo: $code"

# 3) Veredicto global
if ($code -eq 0) {
    Write-Log "VEREDICTO GLOBAL: OK (verificacion de punta a punta)"
} else {
    Write-Log "VEREDICTO GLOBAL: CON FALLOS (revisar lineas [FALLO] de arriba)"
}
Write-Log "=== Fin ==="
