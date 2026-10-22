# SPDX-License-Identifier: GPL-2.0-only
# Read-only validation of a downloaded build before installation can write A:.
param([Parameter(Mandatory)][string]$Build)
$ErrorActionPreference='Stop'
$manifest=Get-Content -LiteralPath (Join-Path $Build 'SHA256SUMS')
function Get-ElfHash([string]$Name){
    $lines=@($manifest | Where-Object {$_ -match "/$([regex]::Escape($Name))$"})
    if($lines.Count -ne 1){throw "ROJO [ELF]: hash ausente o ambiguo de $Name"}
    $sha=($lines[0] -split '\s+')[0]
    if($sha -notmatch '^[0-9a-fA-F]{64}$'){throw 'ROJO [ELF]: hash invalido'}
    return $sha
}
$archive=Join-Path $Build 'elf-produccion.tar.gz'
if(-not(Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive).Hash -ine (Get-ElfHash 'elf-produccion.tar.gz')){
    throw 'ROJO [ELF]: falta una auditoria de produccion con hash correcto; reconstruir y descargar.'
}
foreach($binary in @('init','caja','ultra-raiz','ultra-red-contadores','minas')){
    $text=& tar.exe -xOf $archive "./$binary/informe.json"
    if($LASTEXITCODE -ne 0){throw "ROJO [ELF]: falta el informe de $binary"}
    $report=($text -join "`n") | ConvertFrom-Json
    if($report.Schema -ne 1 -or $report.Status -ne 'PASS' -or $report.Binary -ne $binary -or
       $report.SHA256 -notmatch '^[0-9a-f]{64}$' -or $report.SHA256 -ine (Get-ElfHash $binary)){
        throw "ROJO [ELF]: informe de $binary incompatible con esta construccion"
    }
    foreach($rule in @('NO-WX-LOAD','STACK-RW-NX','RELRO-REQUIRED','DYNAMIC-NOW','NO-DT-NEEDED','NO-TEXTREL','CET-PROPERTY-IBT-SHSTK','MAIN-ENDBR64')){
        if($report.Rules -notcontains $rule){throw "ROJO [ELF]: falta la regla $rule en $binary"}
    }
}
Write-Output 'VERDE [ELF]: cinco ejecutables de produccion auditados y ligados al manifiesto.'
