# SPDX-License-Identifier: GPL-2.0-only
param([Parameter(Mandatory)][string]$Build)
$ErrorActionPreference='Stop'
$validator=Join-Path $PSScriptRoot 'validar-elf-produccion.ps1'
$root=Join-Path (Split-Path $PSScriptRoot) ('build/pruebas-validacion-elf-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
& $validator -Build $Build
function Fixture([string]$Name){
    $path=Join-Path $root $Name
    New-Item -ItemType Directory -Path $path | Out-Null
    Copy-Item -LiteralPath (Join-Path $Build 'elf-produccion.tar.gz') -Destination $path
    Copy-Item -LiteralPath (Join-Path $Build 'SHA256SUMS') -Destination $path
    return $path
}
function Reject([string]$Path,[string]$Pattern){
    try{& $validator -Build $Path}catch{
        if($_.Exception.Message -notmatch $Pattern){throw}
        Write-Output "RECHAZO-OK: $(Split-Path $Path -Leaf)"
        return
    }
    throw "ROJO: se acepto $(Split-Path $Path -Leaf)"
}
$case=Fixture 'archivo-alterado'
[IO.File]::AppendAllText((Join-Path $case 'elf-produccion.tar.gz'),'x')
Reject $case 'hash correcto'
$case=Fixture 'hash-init-distinto'
$manifest=Join-Path $case 'SHA256SUMS'
$lines=Get-Content -LiteralPath $manifest | ForEach-Object {if($_ -match '/init$'){('0'*64)+($_.Substring(64))}else{$_}}
$lines | Set-Content -LiteralPath $manifest -Encoding ascii
Reject $case 'incompatible'
$case=Fixture 'hash-ambiguo'
Add-Content -LiteralPath (Join-Path $case 'SHA256SUMS') -Value (('0'*64)+'  /otra/init') -Encoding ascii
Reject $case 'ambiguo'
foreach($mode in @('recursos-ausente','recursos-distinto')){
    $case=Fixture $mode
    $manifest=Join-Path $case 'SHA256SUMS'
    $lines=Get-Content -LiteralPath $manifest | ForEach-Object {
        if($_ -match '/ultra-recursos-contadores$'){
            if($mode -eq 'recursos-distinto'){('0'*64)+($_.Substring(64))}
        }else{$_}
    }
    $lines | Set-Content -LiteralPath $manifest -Encoding ascii
    Reject $case 'ausente|incompatible'
}
foreach($mode in @('estado-fallido','regla-ausente')){
    $case=Fixture $mode
    $unpack=Join-Path $case 'contenido'
    New-Item -ItemType Directory -Path $unpack | Out-Null
    & tar.exe -xf (Join-Path $case 'elf-produccion.tar.gz') -C $unpack
    if($LASTEXITCODE){throw 'No se pudo preparar fixture'}
    $reportPath=Join-Path $unpack 'init/informe.json'
    $report=Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
    if($mode -eq 'estado-fallido'){$report.Status='FAIL'}else{$report.Rules=@($report.Rules | Where-Object {$_ -ne 'NO-WX-LOAD'})}
    $report | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath $reportPath -Encoding utf8
    & tar.exe -czf (Join-Path $case 'elf-produccion.tar.gz') -C $unpack .
    if($LASTEXITCODE){throw 'No se pudo empaquetar fixture'}
    $sha=(Get-FileHash -LiteralPath (Join-Path $case 'elf-produccion.tar.gz')).Hash.ToLowerInvariant()
    $manifest=Join-Path $case 'SHA256SUMS'
    $lines=Get-Content -LiteralPath $manifest | ForEach-Object {if($_ -match '/elf-produccion\.tar\.gz$'){$sha+($_.Substring(64))}else{$_}}
    $lines | Set-Content -LiteralPath $manifest -Encoding ascii
    Reject $case 'incompatible|falta la regla'
}
Write-Output 'VALIDACION-ELF-OK: control valido y siete rechazos; sin escribir A: ni ejecutar binarios Linux.'
