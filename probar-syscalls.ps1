# SPDX-License-Identifier: GPL-2.0-only
param([ValidateRange(1,999)][int]$Ronda=8)
$ErrorActionPreference='Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $opts=@('-i','vm/builder_ed25519','-o','BatchMode=yes','-o','ConnectTimeout=15',
            '-o','KexAlgorithms=curve25519-sha256','-o','UserKnownHostsFile=vm/known_hosts')
    $guest='builder@127.0.0.1'
    $runId=[guid]::NewGuid().ToString('N')
    $remote="/home/builder/linux-reconstruido/auditoria-syscalls-$runId"
    $runOut="/home/builder/linux-reconstruido/build/auditoria-syscalls/$runId"
    $evidence="evidencias/syscalls/ronda$Ronda-$runId"
    & ssh.exe @opts -p 12222 $guest "mkdir -p $remote"
    if($LASTEXITCODE -ne 0){throw 'No se pudo acceder al taller.'}
    $scripts=@('scripts/catalogo-syscalls.py','scripts/auditar-syscalls.sh','scripts/guardianes.sh','scripts/verificar-seccomp.py',
               'src/rootfs/bin/probar-caja','src/rootfs/etc/os-release')
    & scp.exe @opts -P 12222 @scripts "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Transferencia de auditoria incompleta.'}
    & scp.exe @opts -P 12222 -r src/caja src/guardian "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Transferencia de guardianes incompleta.'}
    & ssh.exe @opts -p 12222 $guest "ULTRA_SYSCALLS_OUT=$runOut bash $remote/auditar-syscalls.sh"
    if($LASTEXITCODE -ne 0){throw 'ROJO: auditoria fallida; no se copian informes anteriores.'}
    $files=@('/home/builder/linux-reconstruido/build/syscalls/informe.txt',
             '/home/builder/linux-reconstruido/build/syscalls/catalogo.csv',
             '/home/builder/linux-reconstruido/build/syscalls/metadatos.json',
             '/home/builder/linux-reconstruido/build/syscalls/busybox.strace',
             '/home/builder/linux-reconstruido/build/syscalls/pid1-abort.txt',
             '/home/builder/linux-reconstruido/build/syscalls/pid1-trap.txt',
             '/home/builder/linux-reconstruido/build/syscalls/formal.txt',
             '/home/builder/linux-reconstruido/build/syscalls/filtro.bpf',
             '/home/builder/linux-reconstruido/build/syscalls/SHA256SUMS',
             "$remote/caja/syscalls-x86_64.h")
    $targets=@('evidencias/syscalls-ronda6.txt','evidencias/syscalls-ronda6.csv',
               'evidencias/syscalls-metadatos-ronda6.json','evidencias/syscalls-busybox-ronda6.strace',
               'evidencias/guardian-pid1-abort-ronda6.txt','evidencias/guardian-pid1-trap-ronda6.txt',
               'evidencias/seccomp-formal-ronda6.txt','evidencias/seccomp-filtro-ronda6.bpf','evidencias/seccomp-SHA256SUMS-ronda6.txt',
               'src/caja/syscalls-x86_64.h')
    # Keep generated headers inside this run; no background audit edits production inputs.
    $files = @($files[0..($files.Count-2)] | ForEach-Object { $_ -replace '/home/builder/linux-reconstruido/build/syscalls', $runOut })
    $targets = @($targets[0..($targets.Count-2)] | ForEach-Object { Join-Path $evidence ((Split-Path $_ -Leaf) -replace 'ronda6', "ronda$Ronda") })
    New-Item -ItemType Directory -Path $evidence -Force | Out-Null
    for($i=0;$i -lt $files.Count;$i++){
        & scp.exe @opts -P 12222 "${guest}:$($files[$i])" $targets[$i]
        if($LASTEXITCODE -ne 0){throw 'No se pudo recuperar una evidencia.'}
    }
    Write-Output "EVIDENCIAS: $evidence"
} finally {
    Pop-Location
}
