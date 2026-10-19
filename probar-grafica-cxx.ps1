# SPDX-License-Identifier: GPL-2.0-only
$ErrorActionPreference='Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $id=[guid]::NewGuid().ToString('N')
    $remote="/home/builder/linux-reconstruido/grafica-cxx-$id"
    $evidence="evidencias/cxx/grafica-$id"
    $opts=@('-i','vm/builder_ed25519','-o','BatchMode=yes','-o','ConnectTimeout=15',
            '-o','KexAlgorithms=curve25519-sha256','-o','UserKnownHostsFile=vm/known_hosts')
    $guest='builder@127.0.0.1'
    & ssh.exe @opts -p 12222 $guest "mkdir -p $remote"
    if($LASTEXITCODE -ne 0){throw 'No se pudo acceder al taller.'}
    & scp.exe @opts -P 12222 -r 'C++' "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Snapshot C++ incompleto.'}
    & scp.exe @opts -P 12222 scripts/aislar-grafica-software.sh "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Aislamiento no copiado.'}
    New-Item -ItemType Directory -Path $evidence -Force | Out-Null
    $commands=@(
        'set -euo pipefail',
        "cd $remote",
        "find 'C++' -type f -print0 | sort -z | xargs -0 sha256sum",
        'sha256sum aislar-grafica-software.sh',
        'g++ --version',
        'pacman -Q',
        'pkg-config --modversion egl glesv2',
        "cmake -S 'C++' -B prod -DCMAKE_CXX_COMPILER=g++ -DULTRA_CXX_EGL=ON",
        'cmake --build prod',
        'cat prod/compile_commands.json',
        'bash aislar-grafica-software.sh prod/laboratorio/sonda-egl',
        "cmake -S 'C++' -B diag -DCMAKE_CXX_COMPILER=g++ -DULTRA_CXX_EGL=ON -DULTRA_CXX_DIAGNOSTICO=ON",
        'cmake --build diag',
        'cat diag/compile_commands.json',
        'bash aislar-grafica-software.sh diag/laboratorio/sonda-egl'
    )
    & ssh.exe @opts -p 12222 $guest ($commands -join '; ') 2>&1 |
        Tee-Object -FilePath "$evidence/informe.txt"
    if($LASTEXITCODE -ne 0){throw "Fallo del banco; conservar evidencia en $evidence"}
    Write-Output "EVIDENCIAS: $evidence"
} finally { Pop-Location }
