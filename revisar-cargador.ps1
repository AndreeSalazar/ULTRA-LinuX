# SPDX-License-Identifier: GPL-2.0-only
$ErrorActionPreference='Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $id=[guid]::NewGuid().ToString('N')
    $remote="/home/builder/linux-reconstruido/revision-cargador-$id"
    $out="/home/builder/linux-reconstruido/build/revision-cargador/$id"
    $evidence="evidencias/revision-cargador/$id"
    $opts=@('-i','vm/builder_ed25519','-o','BatchMode=yes','-o','ConnectTimeout=15',
            '-o','KexAlgorithms=curve25519-sha256','-o','UserKnownHostsFile=vm/known_hosts')
    $guest='builder@127.0.0.1'
    & ssh.exe @opts -p 12222 $guest "mkdir -p $remote"
    if($LASTEXITCODE -ne 0){throw 'No se pudo acceder al taller.'}
    & scp.exe @opts -P 12222 src/hardware.c src/init.h scripts/guardianes.sh scripts/aislar-mina.sh scripts/auditar-cargador.sh "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'No se completo el snapshot.'}
    & scp.exe @opts -P 12222 -r src/guardian "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'No se completaron las herramientas de revision.'}
    & ssh.exe @opts -p 12222 $guest "ULTRA_REVISION_OUT=$out bash $remote/auditar-cargador.sh"
    if($LASTEXITCODE -ne 0){throw 'Fallo la infraestructura de revision.'}
    New-Item -ItemType Directory -Path $evidence -Force | Out-Null
    foreach($name in @('informe.txt','fuentes.sha256')) {
        & scp.exe @opts -P 12222 "${guest}:$out/$name" "$evidence/$name"
        if($LASTEXITCODE -ne 0){throw 'No se pudo recuperar la evidencia.'}
    }
    Write-Output "EVIDENCIAS: $evidence"
} finally { Pop-Location }
