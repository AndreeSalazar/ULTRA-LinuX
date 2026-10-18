# SPDX-License-Identifier: GPL-2.0-only
$ErrorActionPreference='Stop'
$project=$PSScriptRoot
$wsl=Join-Path $env:ProgramFiles 'WSL/wsl.exe'
if(-not(Test-Path -LiteralPath $wsl)){$wsl='wsl.exe'}
& $wsl --version
if($LASTEXITCODE -ne 0){throw 'WSL no está operativo. Completa instalación y reinicio pendiente antes de continuar.'}
$distros=(& $wsl --list --quiet | Out-String).Replace("`0",'')
if($distros -notmatch '(?m)^\s*Debian\s*$'){
    & $wsl --install --distribution Debian --no-launch --web-download
    if($LASTEXITCODE -ne 0){throw 'No se pudo instalar Debian; revisar el mensaje anterior.'}
}
# The Windows folder is mounted by WSL; kernel checkout/build stay on Linux ext4.
$script=Join-Path $project 'scripts/instalar-herramientas.sh'
$linuxScript=(& $wsl -d Debian -u root -- wslpath -a -u $script | Out-String).Trim()
if($LASTEXITCODE -ne 0 -or -not $linuxScript){throw 'Debian no está listo. Comprueba si Windows solicita reiniciar.'}
& $wsl -d Debian -u root -- bash $linuxScript
if($LASTEXITCODE -ne 0){throw 'La preparación Linux falló; revisar antes de compilar.'}