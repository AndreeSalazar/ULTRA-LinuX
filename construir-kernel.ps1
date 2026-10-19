# SPDX-License-Identifier: GPL-2.0-only
param([ValidateSet('Compilar','Estado','Descargar','Ver','VerDisco')][string]$Accion='Estado',
      [ValidateSet('produccion','vigia','generico','pc')][string]$Perfil='produccion')
# Each profile keeps its own build tree and artifacts, so switching never rebuilds the other.
$dirs=@{produccion='tiny64-baseline';vigia='tiny64-vigia';generico='x86_64-generico';pc='x86_64-pc'}
$local=@{produccion='baseline';vigia='vigia';generico='generico';pc='pc'}
$ErrorActionPreference='Stop'
$project=$PSScriptRoot
$key=Join-Path $project 'vm/builder_ed25519'
# ssh splits option values on spaces, and this folder name has one: use a relative path.
Set-Location -LiteralPath $project
$known='UserKnownHostsFile=vm/known_hosts'
# Reject an absent workshop before SSH: build failure is not a compiler failure.
if($Accion -notin @('Ver','VerDisco')){
    $running=$false
    $record=Join-Path $project 'vm/session.json'
    if(Test-Path -LiteralPath $record){
        try{
            $session=Get-Content -LiteralPath $record -Raw | ConvertFrom-Json
            $process=Get-Process -Id ([int]$session.QEMU) -ErrorAction SilentlyContinue
            $running=($null -ne $process -and $process.ProcessName -eq 'qemu-system-x86_64')
        }catch{$running=$false}
    }
    if(-not $running){
        Write-Host 'AMARILLO [TALLER-APAGADO]: QEMU no esta ejecutando el taller. No se intento SSH ni compilacion.' -ForegroundColor Yellow
        Write-Host 'Primero: .\preparar-entorno.ps1 -Accion Iniciar'
        Write-Host 'Si WHPX falla: activar SVM en BIOS, guardar y reiniciar Windows para completar HypervisorPlatform.'
        exit 2
    }
    $client=[Net.Sockets.TcpClient]::new()
    try{$connection=$client.ConnectAsync('127.0.0.1',12222);$ready=$connection.Wait(1500) -and $client.Connected}catch{$ready=$false}finally{$client.Dispose()}
    if(-not $ready){
        Write-Host 'AMARILLO [SSH-NO-LISTO]: el taller existe, pero su acceso local aun no responde. Espera a que termine de arrancar y vuelve a Estado.' -ForegroundColor Yellow
        exit 2
    }
}
function Invoke-Guest([string]$Command){
    & ssh.exe -i $key -p 12222 -o BatchMode=yes -o ConnectTimeout=30 -o KexAlgorithms=curve25519-sha256 -o $known builder@127.0.0.1 $Command
    if($LASTEXITCODE -ne 0){throw 'ROJO: la operacion Linux fallo; revisar su motivo antes de avanzar.'}
}
if($Accion -eq 'Compilar'){
    Invoke-Guest 'mkdir -p /home/builder/linux-reconstruido/lab && rm -rf /home/builder/linux-reconstruido/lab/rootfs /home/builder/linux-reconstruido/lab/guardian /home/builder/linux-reconstruido/lab/caja'
    # BusyBox comes from the official Arch repository; the build rejects it if not static.
    Invoke-Guest 'pacman -Q busybox >/dev/null 2>&1 || sudo pacman -S --noconfirm --needed busybox'
    # erofs-utils builds the read-only system partition of the generic disk.
    Invoke-Guest 'pacman -Q erofs-utils >/dev/null 2>&1 || sudo pacman -S --noconfirm --needed erofs-utils'
    & scp.exe -i $key -P 12222 -o BatchMode=yes -o KexAlgorithms=curve25519-sha256 -o $known (Join-Path $project 'src/init.c') (Join-Path $project 'src/init.h') (Join-Path $project 'src/particiones.c') (Join-Path $project 'src/hardware.c') (Join-Path $project 'src/tarjeta.c') (Join-Path $project 'scripts/build-baseline.sh') (Join-Path $project 'scripts/guardianes.sh') 'builder@127.0.0.1:/home/builder/linux-reconstruido/lab/'
    if($LASTEXITCODE -ne 0){throw 'ROJO: transferencia de fuentes incompleta.'}
    & scp.exe -r -i $key -P 12222 -o BatchMode=yes -o KexAlgorithms=curve25519-sha256 -o $known (Join-Path $project 'src/rootfs') (Join-Path $project 'src/guardian') (Join-Path $project 'src/caja') 'builder@127.0.0.1:/home/builder/linux-reconstruido/lab/'
    if($LASTEXITCODE -ne 0){throw 'ROJO: transferencia del rootfs incompleta.'}
    Invoke-Guest ('if systemctl is-active --quiet linux-reconstruido-build; then echo "La compilacion ya esta en curso"; else sudo systemctl reset-failed linux-reconstruido-build 2>/dev/null || true; sudo systemd-run --collect --unit=linux-reconstruido-build --uid=builder --setenv=HOME=/home/builder /bin/bash /home/builder/linux-reconstruido/lab/build-baseline.sh '+$Perfil+'; fi')
}elseif($Accion -eq 'Estado'){
    Invoke-Guest ('systemctl show linux-reconstruido-build -p ActiveState -p SubState -p ExecMainStatus; tail -n 20 /home/builder/linux-reconstruido/build/'+$dirs[$Perfil]+'/build.log')
}elseif($Accion -eq 'Descargar'){
    # Name the reason: a build still running is not a failed build.
    $ready=& ssh.exe -i $key -p 12222 -o BatchMode=yes -o ConnectTimeout=30 -o KexAlgorithms=curve25519-sha256 -o $known builder@127.0.0.1 ('if test -f /home/builder/linux-reconstruido/artifacts/'+$Perfil+'/build-complete; then echo listo; elif systemctl is-active --quiet linux-reconstruido-build; then echo compilando; else echo falta; fi')
    if($ready -eq 'compilando'){
        Write-Host "AMARILLO [COMPILANDO]: el kernel $Perfil todavia se esta compilando; no se descarga un kernel a medias." -ForegroundColor Yellow
        Write-Host "Vuelve a mirar con: .\construir-kernel.ps1 -Accion Estado -Perfil $Perfil"
        exit 2
    }
    if($ready -ne 'listo'){throw "ROJO [SIN-KERNEL]: no hay kernel $Perfil completo; la ultima compilacion fallo o no se lanzo. Revisar con -Accion Estado -Perfil $Perfil."}
    $out=Join-Path $project ('build/'+$local[$Perfil])
    New-Item -ItemType Directory -Force $out | Out-Null
    $files=@('bzImage-tiny64','tiny64.config','kernelrelease.txt','guardianes.txt','SHA256SUMS')
    if($Perfil -in @('generico','pc')){$files+=@('modulos.tar.gz','ultra-esp.img','disco-bmo.img','sistema-a.erofs','datos.ext4')}
    foreach($file in $files){
        & scp.exe -i $key -P 12222 -o BatchMode=yes -o KexAlgorithms=curve25519-sha256 -o $known ('builder@127.0.0.1:/home/builder/linux-reconstruido/artifacts/'+$Perfil+'/'+$file) $out
        if($LASTEXITCODE -ne 0){throw "ROJO: falta descargar $file"}
    }
    $expected=((Get-Content (Join-Path $out 'SHA256SUMS') -First 1) -split '\s+')[0]
    if((Get-FileHash (Join-Path $out 'bzImage-tiny64')).Hash -ine $expected){throw 'ROJO: hash del kernel descargado incorrecto.'}
    Write-Host 'VERDE: kernel descargado y hash verificado. Arranque todavia pendiente.'
}elseif($Accion -eq 'Ver'){
    $kernel=Join-Path $project ('build/'+$local[$Perfil]+'/bzImage-tiny64')
    if(-not(Test-Path -LiteralPath $kernel)){throw 'AMARILLO: primero completar Compilar y Descargar; aun no hay kernel listo para mostrar.'}
    $expected=((Get-Content (Join-Path $project ('build/'+$local[$Perfil]+'/SHA256SUMS')) -First 1) -split '\s+')[0]
    if((Get-FileHash $kernel).Hash -ine $expected){throw 'ROJO: kernel distinto del artefacto verificado.'}
    $qemu=Join-Path $env:ProgramFiles 'qemu/qemu-system-x86_64.exe'
    & $qemu -name ('ULTRA LinuX - '+$Perfil) -machine pc -accel whpx -cpu host -smp 1 -m 1024 -kernel $kernel -append 'console=ttyS0,115200 console=tty0 rdinit=/init panic=-1 noapic quiet GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK:glibc.cpu.x86_shstk=on' -nic none -display gtk -serial stdio -no-reboot
    if($LASTEXITCODE -ne 0){throw 'ROJO: QEMU no pudo ejecutar el kernel; revisar aceleracion y salida.'}
}elseif($Accion -eq 'VerDisco'){
    # Boots the disk image the way a PC does: UEFI firmware, GPT, ESP, BOOTX64.EFI. No -kernel, no GRUB.
    $dir=Join-Path $project ('build/'+$local[$Perfil])
    $img=Join-Path $dir 'ultra-esp.img'
    if(-not(Test-Path -LiteralPath $img)){throw "AMARILLO [SIN-DISCO]: el perfil $Perfil no tiene ultra-esp.img; usar -Perfil generico tras Compilar y Descargar."}
    $sums=Get-Content (Join-Path $dir 'SHA256SUMS')
    $expected=(($sums | Where-Object {$_ -match 'ultra-esp\.img'}) -split '\s+')[0]
    if((Get-FileHash $img).Hash -ine $expected){throw 'ROJO [HASH]: la imagen de disco no coincide con SHA256SUMS.'}
    $share=Join-Path $env:ProgramFiles 'qemu/share'
    $vars=Join-Path $dir 'ovmf-vars.fd'
    Copy-Item (Join-Path $share 'edk2-i386-vars.fd') $vars -Force
    $qemu=Join-Path $env:ProgramFiles 'qemu/qemu-system-x86_64.exe'
    & $qemu -name ('ULTRA LinuX - disco EFI') -machine pc -accel whpx -cpu host -smp 2 -m 1024 -drive ('if=pflash,format=raw,readonly=on,file='+(Join-Path $share 'edk2-x86_64-code.fd')) -drive ('if=pflash,format=raw,file='+$vars) -drive ('file='+$img+',format=raw,if=virtio,snapshot=on') -nic none -display gtk -serial stdio -no-reboot
    if($LASTEXITCODE -ne 0){throw 'ROJO: QEMU no pudo arrancar el disco; revisar firmware y salida.'}
}
