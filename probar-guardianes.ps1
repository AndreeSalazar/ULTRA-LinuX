# SPDX-License-Identifier: GPL-2.0-only
param([ValidateRange(1,999)][int]$Ronda=7)
$ErrorActionPreference='Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $sshArgs=@('-i','vm/builder_ed25519','-o','BatchMode=yes','-o','ConnectTimeout=15',
               '-o','KexAlgorithms=curve25519-sha256','-o','UserKnownHostsFile=vm/known_hosts')
    $guest='builder@127.0.0.1'
    $remote='/home/builder/linux-reconstruido/auditoria-guardianes'
    & ssh.exe @sshArgs -p 12222 $guest "mkdir -p $remote"
    if($LASTEXITCODE -ne 0){throw 'No se pudo acceder al taller Linux.'}
    $scripts=@('scripts/guardianes.sh','scripts/minas-evaluar.sh','scripts/probar-evaluador.sh',
               'scripts/campo-minas.sh','scripts/cruces-guardianes.sh','scripts/aislar-mina.sh','scripts/campo-hilos.sh','scripts/campo-dominios.sh')
    & scp.exe @sshArgs -P 12222 @scripts "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Transferencia de pruebas incompleta.'}
    & scp.exe @sshArgs -P 12222 -r src/guardian "${guest}:$remote/"
    if($LASTEXITCODE -ne 0){throw 'Transferencia de guardianes incompleta.'}
    & ssh.exe @sshArgs -p 12222 $guest "bash $remote/probar-evaluador.sh && bash $remote/campo-minas.sh && bash $remote/cruces-guardianes.sh && bash $remote/campo-hilos.sh && bash $remote/campo-dominios.sh"
    $testCode=$LASTEXITCODE
    if($testCode -ne 0){throw 'ROJO: auditoria fallida; revisar los registros en el taller. No se copian informes anteriores.'}
    $files=@('/home/builder/linux-reconstruido/build/minas/informe.txt',
             '/home/builder/linux-reconstruido/build/cruces/informe.txt',
             '/home/builder/linux-reconstruido/build/hilos/informe.txt',
             '/home/builder/linux-reconstruido/build/hilos/race.txt',
             '/home/builder/linux-reconstruido/build/hilos/atomic.txt',
             '/home/builder/linux-reconstruido/build/minas/aislamiento.txt',
             '/home/builder/linux-reconstruido/build/dominios/informe.txt')
    $targets=@("evidencias/campo-minas-ronda$Ronda.txt","evidencias/cruces-guardianes-ronda$Ronda.txt","evidencias/hilos-ronda$Ronda.txt",
               "evidencias/hilos-carrera-ronda$Ronda.txt","evidencias/hilos-control-ronda$Ronda.txt","evidencias/aislamiento-ronda$Ronda.txt","evidencias/dominios-ronda$Ronda.txt")
    for($i=0;$i -lt $files.Count;$i++){
        & scp.exe @sshArgs -P 12222 "${guest}:$($files[$i])" $targets[$i]
        if($LASTEXITCODE -ne 0){throw 'No se pudo recuperar el informe de pruebas.'}
    }
} finally {
    Pop-Location
}
