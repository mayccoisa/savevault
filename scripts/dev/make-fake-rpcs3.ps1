# Monta uma pasta de RPCS3 falsa, com PARAM.SFO VALIDO segundo o formato documentado, para
# provar backup e restauracao sem instalar emulador nenhum.
#
# Layout: a maquina emulada mora em dev_hdd0, e o save do usuario em
# dev_hdd0/home/<perfil>/savedata/<TITLE ID + sufixo>/, com os trofeus em .../trophy/.
# O identificador do perfil e criado pelo emulador e MUDA de maquina para maquina: por isso o
# parametro -Profile, que serve para simular a outra maquina na restauracao.
param(
    [Parameter(Mandatory=$true)][string]$Root,
    [string]$Profile = '00000001'
)

$ErrorActionPreference = 'Stop'

function New-ParamSfo {
    param([string]$Title)

    $key = [byte[]][Text.Encoding]::ASCII.GetBytes("TITLE`0")
    $value = [byte[]]([Text.Encoding]::UTF8.GetBytes($Title) + [byte]0)

    $out = New-Object System.Collections.Generic.List[byte]
    $out.AddRange([BitConverter]::GetBytes([uint32]0x46535000))
    $out.AddRange([BitConverter]::GetBytes([uint32]0x00000101))
    $out.AddRange([BitConverter]::GetBytes([uint32]36))
    $out.AddRange([BitConverter]::GetBytes([uint32](36 + $key.Length)))
    $out.AddRange([BitConverter]::GetBytes([uint32]1))
    $out.AddRange([BitConverter]::GetBytes([uint16]0))
    $out.AddRange([BitConverter]::GetBytes([uint16]0x0204))
    $out.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $out.AddRange([BitConverter]::GetBytes([uint32]$value.Length))
    $out.AddRange([BitConverter]::GetBytes([uint32]0))
    $out.AddRange($key)
    $out.AddRange($value)

    return $out.ToArray()
}

$home3 = Join-Path $Root "dev_hdd0\home\$Profile"
foreach ($sub in 'dev_flash', 'dev_hdd1', 'config') {
    New-Item -ItemType Directory -Force -Path (Join-Path $Root $sub) | Out-Null
}
New-Item -ItemType Directory -Force -Path (Join-Path $home3 'savedata') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $home3 'trophy') | Out-Null

# Dois saves do mesmo jogo (automatico e manual), que precisam virar um jogo so.
foreach ($sufixo in '-AUTO', '-SLOT01') {
    $dir = Join-Path $home3 "savedata\BLES00932$sufixo"
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $dir 'PARAM.SFO'), (New-ParamSfo -Title 'DEMONS SOULS'))
    [IO.File]::WriteAllBytes((Join-Path $dir 'SAVEDATA'), [byte[]](1..64))
}

# Outro jogo, digital.
$dir = Join-Path $home3 'savedata\NPEA00275-AUTO'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
[IO.File]::WriteAllBytes((Join-Path $dir 'PARAM.SFO'), (New-ParamSfo -Title 'JOURNEY'))
[IO.File]::WriteAllBytes((Join-Path $dir 'SAVEDATA'), [byte[]](1..32))

# Trofeu: area separada de proposito, para o usuario poder restaurar um sem o outro.
$dir = Join-Path $home3 'trophy\BLES00932_00'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
[IO.File]::WriteAllBytes((Join-Path $dir 'TROPUSR.DAT'), [byte[]](1..48))

# Ruido que NAO e save do usuario.
[IO.File]::WriteAllBytes((Join-Path $Root 'config\config.yml'), [byte[]](1..16))
New-Item -ItemType Directory -Force -Path (Join-Path $Root 'dev_hdd0\game') | Out-Null
[IO.File]::WriteAllBytes((Join-Path $Root 'dev_hdd0\game\dummy.bin'), [byte[]](1..16))

Write-Output "pasta falsa criada em $Root (perfil $Profile)"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
