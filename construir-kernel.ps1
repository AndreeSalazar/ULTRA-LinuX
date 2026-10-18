# SPDX-License-Identifier: GPL-2.0-only
param([ValidateSet('Compilar','Estado','Descargar','Ver')][string]$Accion='Estado')
$ErrorActionPreference='Stop'
$project=$PSScriptRoot
$key=Join-Path $project 'vm/builder_ed25519'
$known='UserKnownHostsFile='+(Join-Path $project 'vm/known_hosts')
function Invoke-Guest([string]$Command){
    & ssh.exe -i $key -p 12222 -o BatchMode=yes -o ConnectTimeout=30 -o KexAlgorithms=curve25519-sha256 -o $known builder@127.0.0.1 $Command
    if($LASTEXITCODE -ne 0){throw 'ROJO: la operacion Linux fallo; revisar su motivo antes de avanzar.'}
}
if($Accion -eq 'Compilar'){
    Invoke-Guest 'mkdir -p /home/builder/linux-reconstruido/lab'
    & scp.exe -i $key -P 12222 -o BatchMode=yes -o KexAlgorithms=curve25519-sha256 -o $known (Join-Path $project 'src/init.c') (Join-Path $project 'scripts/build-baseline.sh') 'builder@127.0.0.1:/home/builder/linux-reconstruido/lab/'
    if($LASTEXITCODE -ne 0){throw 'ROJO: transferencia de fuentes incompleta.'}
    Invoke-Guest 'if systemctl is-active --quiet linux-reconstruido-build; then echo "La compilacion ya esta en curso"; else sudo systemctl reset-failed linux-reconstruido-build 2>/dev/null || true; sudo systemd-run --collect --unit=linux-reconstruido-build --uid=builder --setenv=HOME=/home/builder /bin/bash /home/builder/linux-reconstruido/lab/build-baseline.sh; fi'
}elseif($Accion -eq 'Estado'){
    Invoke-Guest 'systemctl show linux-reconstruido-build -p ActiveState -p SubState -p ExecMainStatus; tail -n 20 /home/builder/linux-reconstruido/build/tiny64-baseline/build.log'
}elseif($Accion -eq 'Descargar'){
    Invoke-Guest 'test -f /home/builder/linux-reconstruido/artifacts/build-complete'
    $out=Join-Path $project 'build/baseline'
    New-Item -ItemType Directory -Force $out | Out-Null
    foreach($file in @('bzImage-tiny64','tiny64.config','kernelrelease.txt','SHA256SUMS')){
        & scp.exe -i $key -P 12222 -o BatchMode=yes -o KexAlgorithms=curve25519-sha256 -o $known ('builder@127.0.0.1:/home/builder/linux-reconstruido/artifacts/'+$file) $out
        if($LASTEXITCODE -ne 0){throw "ROJO: falta descargar $file"}
    }
    $expected=((Get-Content (Join-Path $out 'SHA256SUMS') -First 1) -split '\s+')[0]
    if((Get-FileHash (Join-Path $out 'bzImage-tiny64')).Hash -ine $expected){throw 'ROJO: hash del kernel descargado incorrecto.'}
    Write-Host 'VERDE: kernel descargado y hash verificado. Arranque todavia pendiente.'
}elseif($Accion -eq 'Ver'){
    $kernel=Join-Path $project 'build/baseline/bzImage-tiny64'
    if(-not(Test-Path -LiteralPath $kernel)){throw 'AMARILLO: primero completar Compilar y Descargar; aun no hay kernel listo para mostrar.'}
    $expected=((Get-Content (Join-Path $project 'build/baseline/SHA256SUMS') -First 1) -split '\s+')[0]
    if((Get-FileHash $kernel).Hash -ine $expected){throw 'ROJO: kernel distinto del artefacto verificado.'}
    $qemu=Join-Path $env:ProgramFiles 'qemu/qemu-system-x86_64.exe'
    & $qemu -name 'Linux Reconstruido - baseline GCC' -machine pc -accel whpx -cpu host -smp 1 -m 512 -kernel $kernel -append 'console=ttyS0,115200 console=tty0 rdinit=/init panic=-1 noapic' -nic none -display gtk -serial stdio -no-reboot
    if($LASTEXITCODE -ne 0){throw 'ROJO: QEMU no pudo ejecutar el kernel; revisar aceleracion y salida.'}
}