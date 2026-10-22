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
    & scp.exe @opts -P 12222 scripts/aislar-grafica-software.sh scripts/ejecutar-grafica-limitada.sh "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Aislamiento no copiado.'}
    New-Item -ItemType Directory -Path $evidence -Force | Out-Null
    $commands=@(
        'set -euo pipefail',
        "cd $remote",
        "find 'C++' -type f -print0 | sort -z | xargs -0 sha256sum",
        'sha256sum aislar-grafica-software.sh ejecutar-grafica-limitada.sh',
        'g++ --version',
        'pacman -Q',
        'pkg-config --modversion egl glesv2',
        "cmake -S 'C++' -B prod -DCMAKE_CXX_COMPILER=g++ -DULTRA_CXX_EGL=ON -DULTRA_CXX_VULKAN=ON",
        'cmake --build prod',
        'cat prod/compile_commands.json',
        'bash ejecutar-grafica-limitada.sh prod/laboratorio/probar-contratos',
        'bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-egl',
        'bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-gpus',
        'bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-vulkan',
        'rc=0; bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-vulkan --control-error-final > control-error-final.txt 2>&1 || rc=$?; cat control-error-final.txt; test "$rc" = 1; grep -qx "VULKAN errores=2 esperados=1 inesperados=1" control-error-final.txt; echo "VERDE [V-03]: error tardio convierte la ejecucion en rechazo"',
        "cmake -S 'C++' -B diag -DCMAKE_CXX_COMPILER=g++ -DULTRA_CXX_EGL=ON -DULTRA_CXX_VULKAN=ON -DULTRA_CXX_DIAGNOSTICO=ON",
        'cmake --build diag',
        'cat diag/compile_commands.json',
        'bash ejecutar-grafica-limitada.sh diag/laboratorio/probar-contratos',
        'bash ejecutar-grafica-limitada.sh diag/laboratorio/sonda-egl',
        'bash ejecutar-grafica-limitada.sh diag/laboratorio/sonda-gpus',
        'bash ejecutar-grafica-limitada.sh diag/laboratorio/sonda-vulkan',
        'for api in egl vulkan; do rc=0; bash ejecutar-grafica-limitada.sh "diag/laboratorio/sonda-$api" --medir > "rechazo-medicion-$api.txt" 2>&1 || rc=$?; cat "rechazo-medicion-$api.txt"; test "$rc" = 2; grep -qx "ROJO \[MEDICION\]: compilacion instrumentada rechazada" "rechazo-medicion-$api.txt"; done',
        'echo "BENCH_COLUMNAS,api,ronda,ruta,wall_ns,cpu_us,rss_pico_proceso_kib"',
        'for run in 1 2 3; do echo "BENCH_EJECUCION,$run"; bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-egl --medir; bash ejecutar-grafica-limitada.sh prod/laboratorio/sonda-vulkan --medir; done'
    )
    & ssh.exe @opts -p 12222 $guest ($commands -join '; ') 2>&1 |
        Tee-Object -FilePath "$evidence/informe.txt"
    if($LASTEXITCODE -ne 0){throw "Fallo del banco; conservar evidencia en $evidence"}
    Write-Output "EVIDENCIAS: $evidence"
} finally { Pop-Location }
