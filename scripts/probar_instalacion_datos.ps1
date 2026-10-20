# SPDX-License-Identifier: GPL-2.0-only
# Exercises the real installer on a temporary filesystem drive; no disks/EFI APIs.
$ErrorActionPreference='Stop'
$project=Split-Path $PSScriptRoot
$fixture=Join-Path $project ('build/prueba-instalador-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force "$fixture/build/generico","$fixture/destino"|Out-Null
Copy-Item -LiteralPath "$project/instalar-en-A.ps1" -Destination "$fixture/instalar-en-A.ps1"
$letter=@('Z','Y','X','W','V')|Where-Object {-not(Get-PSDrive -Name $_ -ErrorAction SilentlyContinue)}|Select-Object -First 1
if(-not $letter){throw 'Sin letra temporal disponible'}
New-PSDrive -Name $letter -PSProvider FileSystem -Root "$fixture/destino"|Out-Null
function Get-Volume {param($DriveLetter) [pscustomobject]@{FileSystem='FAT32';FileSystemLabel='BMO';SizeRemaining=10GB}}
function Get-Partition {param($DriveLetter) [pscustomobject]@{IsSystem=$false;IsBoot=$false;Type='Basic';DiskNumber=-1;PartitionNumber=-1}}
function Assert($ok,$label){if(-not $ok){throw "FALLO: $label"};Write-Output "VERDE: $label"}
function Rechaza([hashtable]$options){
    $caught=$false
    try {& "$fixture/instalar-en-A.ps1" -Accion Copiar -Unidad $letter @options|Out-Null}catch{
        if($_.Exception.Message -notmatch '^ROJO \[DATOS\]'){throw}
        $caught=$true
    }
    Assert $caught 'rechaza antes de escribir'
    Assert ((Get-FileHash "$fixture/destino/EFI/ULTRA/ULTRA.EFI").Hash -eq $script:kernelBefore) 'kernel intacto tras rechazo'
}
try {
    Set-Content "$fixture/build/generico/kernelrelease.txt" 'prueba'
    $hashes=foreach($name in 'bzImage-tiny64','sistema-a.erofs','datos.ext4'){
        Set-Content "$fixture/build/generico/$name" "semilla-$name"
        '{0}  /laboratorio/{1}' -f (Get-FileHash "$fixture/build/generico/$name").Hash,$name
    }
    Set-Content "$fixture/build/generico/SHA256SUMS" $hashes
    & "$fixture/instalar-en-A.ps1" -Accion Copiar -Unidad $letter
    Assert (Test-Path "$fixture/build/datos-$letter.sha256") 'primera instalacion registra presencia esperada'
    $data="$fixture/destino/ULTRA/datos.ext4"
    New-Item -ItemType Directory -Force "$fixture/destino/ULTRA/informe"|Out-Null
    Set-Content "$fixture/destino/ULTRA/informe/arranques.txt" 'informe de prueba'
    Set-Content $data 'datos escritos por Linux despues del arranque'
    $expected=(Get-FileHash $data).Hash
    & "$fixture/instalar-en-A.ps1" -Accion Copiar -Unidad $letter
    Assert ((Get-FileHash $data).Hash -eq $expected) 'actualizacion conserva datos legitimos modificados'
    Assert ((Get-FileHash "$fixture/build/respaldo-datos/$expected.ext4").Hash -eq $expected) 'respaldo completo y verificado'
    $savedReport=Get-ChildItem "$fixture/build/respaldo-informes" -Recurse -Filter arranques.txt|Select-Object -First 1
    Assert ($savedReport -and (Get-FileHash -LiteralPath $savedReport.FullName).Hash -eq (Get-FileHash "$fixture/destino/ULTRA/informe/arranques.txt").Hash) 'informe original respaldado en proyecto Windows'
    $script:kernelBefore=(Get-FileHash "$fixture/destino/EFI/ULTRA/ULTRA.EFI").Hash
    Move-Item -LiteralPath $data -Destination "$fixture/datos-apartados.ext4"
    Rechaza @{}
    Assert (-not(Test-Path $data)) 'no crea una semilla si faltan datos anteriores'
    Move-Item -LiteralPath "$fixture/build/datos-$letter.sha256" -Destination "$fixture/recibo-apartado.sha256"
    Rechaza @{}
    Assert (-not(Test-Path $data)) 'reconoce instalacion previa aunque falte el recibo'
    Move-Item -LiteralPath "$fixture/recibo-apartado.sha256" -Destination "$fixture/build/datos-$letter.sha256"
    Move-Item -LiteralPath "$fixture/datos-apartados.ext4" -Destination $data
    Rechaza @{DatosEsperadosSHA256=('0'*64)}
    Rechaza @{DatosEsperadosSHA256=$expected;NuevosDatos=$true}
    & "$fixture/instalar-en-A.ps1" -Accion Copiar -Unidad $letter -DatosEsperadosSHA256 $expected
    Assert ((Get-FileHash $data).Hash -eq $expected) 'hash esperado correcto permite actualizar'
    Set-Content $data 'ultimos datos antes del reinicio intencional'
    $resetHash=(Get-FileHash $data).Hash
    & "$fixture/instalar-en-A.ps1" -Accion Copiar -Unidad $letter -NuevosDatos
    Assert ((Get-FileHash $data).Hash -eq (Get-FileHash "$fixture/build/generico/datos.ext4").Hash) 'reinicio de datos solo con opcion explicita'
    Assert ((Get-FileHash "$fixture/build/respaldo-datos/$resetHash.ext4").Hash -eq $resetHash) 'reinicio intencional conserva el ultimo original en Windows'
} finally {
    Remove-PSDrive -Name $letter
}
