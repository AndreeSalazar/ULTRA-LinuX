# SPDX-License-Identifier: GPL-2.0-only
$ErrorActionPreference='Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $id=[guid]::NewGuid().ToString('N')
    $remote="/home/builder/linux-reconstruido/pantallas-$id"
    $evidence="evidencias/pantallas/$id"
    $opts=@('-i','vm/builder_ed25519','-o','BatchMode=yes','-o','ConnectTimeout=15',
            '-o','KexAlgorithms=curve25519-sha256','-o','UserKnownHostsFile=vm/known_hosts')
    $guest='builder@127.0.0.1'
    & ssh.exe @opts -p 12222 $guest "mkdir -p $remote"
    if($LASTEXITCODE -ne 0){throw 'Taller no disponible.'}
    & scp.exe @opts -P 12222 src/rootfs/bin/ultra-pantallas scripts/probar-pantallas.py "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Snapshot incompleto.'}
    New-Item -ItemType Directory -Path $evidence -Force | Out-Null
    $commands=@('set -euo pipefail',"cd $remote",'sha256sum ultra-pantallas probar-pantallas.py',
                'python --version','python probar-pantallas.py ./ultra-pantallas')
    & ssh.exe @opts -p 12222 $guest ($commands -join '; ') 2>&1 | Tee-Object -FilePath "$evidence/informe.txt"
    if($LASTEXITCODE -ne 0){throw "Fallo; conservar evidencia: $evidence"}
    Write-Output "EVIDENCIAS: $evidence"
} finally { Pop-Location }
