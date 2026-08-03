# Monta um memory stick de PPSSPP falso, com PARAM.SFO VALIDO segundo o formato documentado,
# para provar backup e restauracao sem instalar emulador nenhum.
#
# Layout tirado do codigo do proprio PPSSPP: PSP/SAVEDATA guarda os saves (uma pasta por save,
# comecando com o id do jogo), PSP/PPSSPP_STATE os estados salvos, PSP/SYSTEM a configuracao.
param([Parameter(Mandatory=$true)][string]$Root)

$ErrorActionPreference = 'Stop'

function New-ParamSfo {
    param([string]$Title)

    $key = [byte[]][Text.Encoding]::ASCII.GetBytes("TITLE`0")
    $value = [byte[]]([Text.Encoding]::UTF8.GetBytes($Title) + [byte]0)

    $out = New-Object System.Collections.Generic.List[byte]
    $out.AddRange([BitConverter]::GetBytes([uint32]0x46535000))   # magica "\0PSF"
    $out.AddRange([BitConverter]::GetBytes([uint32]0x00000101))   # versao
    $out.AddRange([BitConverter]::GetBytes([uint32]36))           # inicio da tabela de chaves
    $out.AddRange([BitConverter]::GetBytes([uint32](36 + $key.Length)))  # inicio da tabela de dados
    $out.AddRange([BitConverter]::GetBytes([uint32]1))            # uma entrada
    $out.AddRange([BitConverter]::GetBytes([uint16]0))            # chave em 0
    $out.AddRange([BitConverter]::GetBytes([uint16]0x0204))       # texto UTF-8
    $out.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $out.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $out.AddRange([BitConverter]::GetBytes([uint32]0))            # dado em 0
    $out.AddRange($key)
    $out.AddRange($value)

    return $out.ToArray()
}

foreach ($sub in 'PSP\SAVEDATA', 'PSP\PPSSPP_STATE', 'PSP\SYSTEM', 'PSP\GAME') {
    New-Item -ItemType Directory -Force -Path (Join-Path $Root $sub) | Out-Null
}

Set-Content -Path (Join-Path $Root 'PSP\SYSTEM\ppsspp.ini') -Value "[General]" -NoNewline

# Dois slots do MESMO jogo: precisam virar um jogo so, senao o usuario restaura metade do
# progresso achando que restaurou tudo.
foreach ($slot in '01', '02') {
    $dir = Join-Path $Root "PSP\SAVEDATA\ULUS1234501".Replace('01', $slot)
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $dir 'PARAM.SFO'), (New-ParamSfo -Title 'MONSTER HUNTER FREEDOM UNITE'))
    [IO.File]::WriteAllBytes((Join-Path $dir 'DATA.BIN'), [byte[]](1..64))
}

# Outro jogo, com nome em japones, para provar que o titulo sai do arquivo e nao do nome da pasta.
$dir = Join-Path $Root 'PSP\SAVEDATA\ULJM0555700'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
[IO.File]::WriteAllBytes((Join-Path $dir 'PARAM.SFO'), (New-ParamSfo -Title 'ペルソナ3 ポータブル'))
[IO.File]::WriteAllBytes((Join-Path $dir 'DATA.BIN'), [byte[]](1..32))

# Save sem metadado: mantem a identidade, perde so o nome bonito.
$dir = Join-Path $Root 'PSP\SAVEDATA\NPUH1000001'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
[IO.File]::WriteAllBytes((Join-Path $dir 'DATA.BIN'), [byte[]](1..16))

# Estado salvo com a miniatura que o emulador mostra na lista.
[IO.File]::WriteAllBytes((Join-Path $Root 'PSP\PPSSPP_STATE\ULUS12345_1.00_1.ppst'), [byte[]](1..64))
[IO.File]::WriteAllBytes((Join-Path $Root 'PSP\PPSSPP_STATE\ULUS12345_1.00_1.jpg'), [byte[]](1..16))

# Ruido que NAO e save do usuario.
New-Item -ItemType Directory -Force -Path (Join-Path $Root 'PSP\GAME\HOMEBREW') | Out-Null
[IO.File]::WriteAllBytes((Join-Path $Root 'PSP\GAME\HOMEBREW\EBOOT.PBP'), [byte[]](1..32))

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
