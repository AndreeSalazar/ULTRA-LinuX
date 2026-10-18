# SPDX-License-Identifier: GPL-2.0-only
param([ValidateSet('Preparar','Iniciar','Estado')][string]$Accion='Estado')
$ErrorActionPreference='Stop'
$python=Join-Path $env:USERPROFILE '.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
$actions=@{Preparar='prepare';Iniciar='start';Estado='status'}
& $python (Join-Path $PSScriptRoot 'scripts/taller_qemu.py') $actions[$Accion]
if($LASTEXITCODE -ne 0){throw 'Revisar el estado del taller QEMU; no se considera preparado todavía.'}