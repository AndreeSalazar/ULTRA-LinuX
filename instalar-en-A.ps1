# SPDX-License-Identifier: GPL-2.0-only
# Installs ULTRA LinuX as files on the FAT32 partition labelled BMO (A:\), without
# touching any partition table, the Windows EFI partition or other drives.
#   Copiar         kernel and system image; datos.ext4 only if it does not exist yet
#   CrearEntrada   adds "ULTRA LinuX" to the board's F11 menu (a firmware boot entry)
#   QuitarEntrada  removes that entry again
#   Estado         shows what is installed and whether the entry exists
# -Perfil pc installs the Zen 3 kernel beside generico: its own .EFI, its own system
# image (named after uname -r) and its own F11 entry; datos.ext4 is shared.
param([ValidateSet('Copiar','CrearEntrada','QuitarEntrada','Estado')][string]$Accion='Estado',
      [ValidateSet('generico','pc')][string]$Perfil='generico',
      [ValidatePattern('^[A-Za-z]$')][string]$Unidad='A', [switch]$NuevosDatos,
      [ValidatePattern('^[0-9a-fA-F]{64}$')][string]$DatosEsperadosSHA256)
$ErrorActionPreference='Stop'
$project=$PSScriptRoot
$build=Join-Path $project "build/$Perfil"
$record=Join-Path $build 'entrada-f11.txt'
$root="${Unidad}:\"
$efi=@{generico='EFI\ULTRA\ULTRA.EFI';pc='EFI\ULTRA\ULTRA-PC.EFI'}[$Perfil]
$title=@{generico='ULTRA LinuX';pc='ULTRA LinuX PC (Zen 3)'}[$Perfil]
$release=(Get-Content -LiteralPath (Join-Path $build 'kernelrelease.txt') -First 1).Trim()
if($release -notmatch '^[0-9A-Za-z.+_-]+$'){throw "ROJO: kernelrelease.txt de $Perfil no es valido"}
$files=[ordered]@{'bzImage-tiny64'=$efi;'sistema-a.erofs'="ULTRA\sistema-$release.erofs";'datos.ext4'='ULTRA\datos.ext4'}

function Test-Destino {
    # Guardians: only a FAT32 data partition labelled BMO, never a system or boot partition.
    $vol=Get-Volume -DriveLetter $Unidad -ErrorAction SilentlyContinue
    if(-not $vol){throw "ROJO [DESTINO]: no existe la unidad ${Unidad}:"}
    if($vol.FileSystem -ne 'FAT32' -or $vol.FileSystemLabel -ne 'BMO'){throw "ROJO [DESTINO]: ${Unidad}: es $($vol.FileSystem) '$($vol.FileSystemLabel)'; se esperaba FAT32 'BMO'"}
    $part=Get-Partition -DriveLetter $Unidad
    if($part.IsSystem -or $part.IsBoot -or $part.Type -ne 'Basic'){throw "ROJO [DESTINO]: ${Unidad}: es particion de sistema o de arranque; no se toca"}
    return $part
}
function Get-Esperado([string]$name){
    $line=Get-Content (Join-Path $build 'SHA256SUMS') | Where-Object { $_ -match "/$([regex]::Escape($name))$" }
    if(-not $line){throw "ROJO [HASH]: $name no esta en SHA256SUMS; volver a descargar"}
    return ($line -split '\s+')[0]
}
function Get-Entrada {
    if(-not(Test-Path -LiteralPath $record)){return $null}
    $id=(Get-Content -LiteralPath $record -First 1).Trim()
    if($id -notmatch '^\{[0-9a-fA-F-]{36}\}$'){throw "ROJO: $record no contiene un identificador valido"}
    return $id
}

if($Accion -eq 'Copiar'){
    $part=Test-Destino
    # Keep the receipt outside the removable drive, so removing ULTRA cannot
    # silently turn an update into a first installation with an empty seed.
    $receipt=Join-Path $project "build/datos-$Unidad.sha256"
    if($NuevosDatos -and $DatosEsperadosSHA256){throw 'ROJO [DATOS]: no combinar -NuevosDatos con -DatosEsperadosSHA256'}
    $expectedData=$DatosEsperadosSHA256
    if(-not $expectedData -and -not $NuevosDatos -and (Test-Path -LiteralPath $receipt)){
        $expectedData=(Get-Content -LiteralPath $receipt -Raw).Trim()
        if($expectedData -notmatch '^[0-9a-fA-F]{64}$'){throw 'ROJO [DATOS]: recibo local invalido'}
    }
    $dataPath=Join-Path $root 'ULTRA\datos.ext4'
    $dataBefore=$null
    if(Test-Path -LiteralPath $dataPath){$dataBefore=(Get-FileHash -LiteralPath $dataPath -Algorithm SHA256).Hash}
    $hasInstallation=(Test-Path -LiteralPath (Join-Path $root 'EFI\ULTRA\ULTRA.EFI')) -or
                     (Test-Path -LiteralPath (Join-Path $root 'EFI\ULTRA\ULTRA-PC.EFI')) -or
                     [bool](Get-Entrada)
    # A saved volume legitimately changes after Linux writes to it. The receipt
    # requires its presence; only an explicit expected hash requires exact bytes.
    if(-not $NuevosDatos -and -not $dataBefore -and ($expectedData -or $hasInstallation)){throw 'ROJO [DATOS]: faltan los datos de la instalacion anterior; restaurarlos antes de copiar (o usar -NuevosDatos intencionadamente)'}
    if($DatosEsperadosSHA256 -and $dataBefore -ine $DatosEsperadosSHA256){throw 'ROJO [DATOS]: datos.ext4 no coincide con el hash esperado; no se ha escrito nada'}
    # Windows project is the master copy, including before an intentional reset.
    if($dataBefore){
        $backup=Join-Path $project "build/respaldo-datos/$dataBefore.ext4"
        New-Item -ItemType Directory -Force (Split-Path $backup)|Out-Null
        if(-not(Test-Path -LiteralPath $backup)){Copy-Item -LiteralPath $dataPath -Destination $backup}
        if((Get-FileHash -LiteralPath $backup -Algorithm SHA256).Hash -ine $dataBefore){throw 'ROJO [DATOS]: respaldo local incorrecto; no se ha escrito en destino'}
    }
    $reportsPath=Join-Path $root 'ULTRA\informe'
    if(Test-Path -LiteralPath $reportsPath){
        $reportBackup=Join-Path $project ("build/respaldo-informes/"+[DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')+'-'+[guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force $reportBackup|Out-Null
        $reportHashes=[ordered]@{}
        foreach($report in Get-ChildItem -LiteralPath $reportsPath -File -Filter '*.txt'){
            $sha=(Get-FileHash -LiteralPath $report.FullName -Algorithm SHA256).Hash
            $saved=Join-Path $reportBackup $report.Name
            Copy-Item -LiteralPath $report.FullName -Destination $saved
            if((Get-FileHash -LiteralPath $saved -Algorithm SHA256).Hash -ine $sha){throw 'ROJO [INFORMES]: respaldo local incorrecto; no se ha escrito en destino'}
            $reportHashes[$report.Name]=$sha
        }
        $reportHashes|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $reportBackup 'SHA256.json') -Encoding utf8
        Write-Host "VERDE [RESPALDO]: informes guardados en $reportBackup"
    }
    # Leftovers of earlier layouts, by exact name only: the shared system image from before
    # each kernel had its own, and the reports that now live in ULTRA\informe.
    foreach($old in 'ULTRA\sistema-a.erofs','ULTRA\PC-configuracion.txt','ULTRA\arranques.txt'){
        $path=Join-Path $root $old
        if(Test-Path -LiteralPath $path){Remove-Item -LiteralPath $path; Write-Host "VERDE [LIMPIEZA]: $path ya no se usa; borrado"}
    }
    New-Item -ItemType Directory -Force (Join-Path $root 'ULTRA') | Out-Null
    $leeme=@'
ULTRA LinuX en esta particion (BMO, A:\ en Windows)
====================================================

EFI\ULTRA\ULTRA.EFI       kernel generico      -> F11: "ULTRA LinuX"
EFI\ULTRA\ULTRA-PC.EFI    kernel Zen 3 (pc)    -> F11: "ULTRA LinuX PC (Zen 3)"

ULTRA\sistema-<version>.erofs   sistema y modulos de cada kernel, solo lectura.
                                 Sin "-pc-": generico. Con "-pc-": Zen 3.
ULTRA\datos.ext4                /datos, compartido por los dos kernels.
                                 Windows no lo lee; no borrarlo.

ULTRA\informe\                  lo que ULTRA deja para leer desde Windows,
                                 actualizado en cada arranque:
  PC-configuracion.txt   componentes y modulos de este PC
                         (escrito en el primer arranque; no cambia
                          salvo que cambie el hardware)
  arranques.txt          una linea por arranque: kernel y tiempos
  dmesg-<version>.txt    registro del kernel del ultimo arranque de
                         cada kernel, con sus marcas de tiempo

Apagar siempre con "poweroff" dentro de ULTRA.
"e" abre el menu con teclado. En Hyprland, Super+Shift+G abre graficos.
Super+Shift+E abre Windows NVMe en /windows, solo lectura.
Instalado y actualizado por instalar-en-A.ps1 (proyecto Linux Reconstruido).
'@
    [IO.File]::WriteAllText((Join-Path (Convert-Path -LiteralPath (Join-Path $root 'ULTRA')) 'LEEME.txt'),($leeme -replace "`r?`n","`r`n"),(New-Object Text.UTF8Encoding $false))
    foreach($name in $files.Keys){
        $src=Join-Path $build $name
        if((Get-FileHash $src).Hash -ine (Get-Esperado $name)){throw "ROJO [HASH]: $name no coincide con SHA256SUMS"}
    }
    $need=($files.Keys | ForEach-Object { (Get-Item (Join-Path $build $_)).Length } | Measure-Object -Sum).Sum
    if((Get-Volume -DriveLetter $Unidad).SizeRemaining -lt $need){throw "ROJO [ESPACIO]: ${Unidad}: no tiene $([math]::Round($need/1MB)) MiB libres"}
    foreach($name in $files.Keys){
        $dst=Join-Path $root $files[$name]
        # /datos lives in datos.ext4: replacing it would erase PC.configuraciÃ³n and everything saved.
        if($name -eq 'datos.ext4' -and (Test-Path -LiteralPath $dst) -and -not $NuevosDatos){
            Write-Host "AMARILLO [DATOS]: se conserva $dst (usar -NuevosDatos para empezar de cero)" -ForegroundColor Yellow
            continue
        }
        New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
        Copy-Item -LiteralPath (Join-Path $build $name) -Destination $dst -Force
        if((Get-FileHash $dst).Hash -ine (Get-Esperado $name)){throw "ROJO [COPIA]: $dst no quedo igual que el original"}
        Write-Host "VERDE [COPIA]: $dst"
    }
    $dataAfter=(Get-FileHash -LiteralPath $dataPath -Algorithm SHA256).Hash
    if($dataBefore -and -not $NuevosDatos -and $dataAfter -ine $dataBefore){throw 'ROJO [DATOS]: los datos cambiaron durante la copia; comprobar el respaldo local'}
    Set-Content -LiteralPath $receipt -Value $dataAfter -Encoding ascii
    $next=if(Get-Entrada){"elegir $title en F11"}else{'-Accion CrearEntrada'}
    Write-Host "VERDE: $title ($release) instalado en ${Unidad}:\ (disco $($part.DiskNumber), particion $($part.PartitionNumber)). Siguiente: $next"
}elseif($Accion -eq 'CrearEntrada'){
    Test-Destino | Out-Null
    if(-not(Test-Path -LiteralPath (Join-Path $root $efi))){throw "AMARILLO: primero -Accion Copiar -Perfil $Perfil"}
    if(Get-Entrada){throw "AMARILLO: la entrada ya existe ($(Get-Entrada)); usar -Accion QuitarEntrada antes de crear otra"}
    # A copy of {bootmgr} is a firmware application entry; Windows writes it to the board's NVRAM.
    $out=(& bcdedit.exe /copy '{bootmgr}' /d $title) -join ' '
    if($LASTEXITCODE -ne 0 -or $out -notmatch '(\{[0-9a-fA-F-]{36}\})'){throw "ROJO [BCD]: bcdedit /copy fallo: $out"}
    $id=$Matches[1]
    Set-Content -LiteralPath $record -Value $id -Encoding ascii
    foreach($set in @(@('device',"partition=${Unidad}:"),@('path',"\$efi"))){
        & bcdedit.exe /set $id $set[0] $set[1] | Out-Null
        if($LASTEXITCODE -ne 0){& bcdedit.exe /delete $id | Out-Null; Remove-Item -LiteralPath $record; throw "ROJO [BCD]: no se pudo fijar $($set[0]); entrada borrada"}
    }
    # Last in the list: Windows stays the default boot; ULTRA is chosen from F11.
    & bcdedit.exe /set '{fwbootmgr}' displayorder $id /addlast | Out-Null
    if($LASTEXITCODE -ne 0){& bcdedit.exe /delete $id | Out-Null; Remove-Item -LiteralPath $record; throw 'ROJO [BCD]: no se pudo anadir al menu del firmware; entrada borrada'}
    Write-Host "VERDE: entrada '$title' $id creada, al final del orden de arranque. Windows sigue siendo el predeterminado."
}elseif($Accion -eq 'QuitarEntrada'){
    $id=Get-Entrada
    if(-not $id){Write-Host 'AMARILLO: no hay entrada registrada por este script.' -ForegroundColor Yellow; exit 0}
    & bcdedit.exe /delete $id | Out-Null
    if($LASTEXITCODE -ne 0){throw "ROJO [BCD]: no se pudo borrar $id"}
    Remove-Item -LiteralPath $record
    Write-Host "VERDE: entrada $id borrada."
}else{
    Test-Destino | Out-Null
    foreach($name in $files.Keys){
        $dst=Join-Path $root $files[$name]
        if(Test-Path -LiteralPath $dst){"{0,-28} {1,8} KiB" -f $files[$name],[math]::Round((Get-Item $dst).Length/1KB)}else{"{0,-28} falta" -f $files[$name]}
    }
    foreach($extra in 'ULTRA\informe\PC-configuracion.txt','ULTRA\informe\arranques.txt'){ if(Test-Path -LiteralPath (Join-Path $root $extra)){"{0,-28} escrito por ULTRA" -f $extra} }
    $id=Get-Entrada
    if($id){& bcdedit.exe /enum $id}else{'Entrada F11: no creada'}
}
