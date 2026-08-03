# Monta uma pasta de dados de shadPS4 falsa, para provar backup e restauracao sem instalar
# emulador nenhum. Fonte do layout: codigo do proprio shadPS4.
#   - pasta de dados: %APPDATA%\shadPS4, ou a subpasta `user` ao lado do executavel (portatil)
#   - save:   home\<id do usuario>\savedata\<serial CUSA>\<nome do diretorio>\
#   - metadado do save: <nome do diretorio>\sce_sys\param.sfo
#   - trofeu: home\<id do usuario>\trophy\<NPWR...>.xml
#
# -User simula OUTRA maquina: o id do usuario e do emulador de la, nao do backup.
# -Empty cria a instalacao sem save, para provar restauracao.
param(
    [Parameter(Mandatory=$true)][string]$Root,
    [string]$User = '1',
    [switch]$Empty
)

$ErrorActionPreference = 'Stop'

# Pastas criadas pelo emulador na inicializacao. Sao elas que marcam a INSTALACAO, e por isso
# continuam aqui depois de o save sumir.
foreach ($sub in 'shader', 'sys_modules', 'log', 'data', 'cache') {
    New-Item -ItemType Directory -Force -Path "$Root\$sub" | Out-Null
}

$home_user = "$Root\home\$User"
New-Item -ItemType Directory -Force -Path "$home_user\savedata" | Out-Null
New-Item -ItemType Directory -Force -Path "$home_user\trophy" | Out-Null

# Um PARAM.SFO minimo, com uma entrada TITLE de texto. Formato: cabecalho de 20 bytes, indice de
# 16 bytes por entrada, depois as tabelas de chave e de valor.
function New-ParamSfo([string]$Title) {
    $key = [Text.Encoding]::ASCII.GetBytes("TITLE`0")
    $value = [Text.Encoding]::UTF8.GetBytes("$Title`0")
    $keyTable = 20 + 16
    $valueTable = $keyTable + $key.Length
    $bytes = New-Object 'System.Collections.Generic.List[byte]'
    $bytes.AddRange([byte[]](0x00, 0x50, 0x53, 0x46))          # magico
    $bytes.AddRange([BitConverter]::GetBytes([uint32]0x0101))   # versao
    $bytes.AddRange([BitConverter]::GetBytes([uint32]$keyTable))
    $bytes.AddRange([BitConverter]::GetBytes([uint32]$valueTable))
    $bytes.AddRange([BitConverter]::GetBytes([uint32]1))        # uma entrada
    $bytes.AddRange([BitConverter]::GetBytes([uint16]0))        # deslocamento da chave
    $bytes.AddRange([BitConverter]::GetBytes([uint16]0x0204))   # formato: texto
    $bytes.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $bytes.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $bytes.AddRange([BitConverter]::GetBytes([uint32]0))        # deslocamento do valor
    $bytes.AddRange($key)
    $bytes.AddRange($value)
    return $bytes.ToArray()
}

if (-not $Empty) {
    $jogos = @{ 'CUSA00207' = 'BLOODBORNE'; 'CUSA03041' = 'GOD OF WAR' }
    foreach ($cusa in $jogos.Keys) {
        $save = "$home_user\savedata\$cusa\SPRJ0005"
        New-Item -ItemType Directory -Force -Path "$save\sce_sys" | Out-Null
        [IO.File]::WriteAllBytes("$save\sce_sys\param.sfo", (New-ParamSfo $jogos[$cusa]))
        [IO.File]::WriteAllBytes("$save\userdata0", [byte[]](1..64))
    }

    # Progresso de trofeu do usuario. Area separada de proposito.
    Set-Content -Path "$home_user\trophy\NPWR12345_00.xml" -Value "<trophy/>" -NoNewline
}

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
