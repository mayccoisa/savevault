# Monta uma pasta de dados de Eden falsa, para provar backup e restauracao sem instalar
# emulador nenhum. O Eden e um fork da linhagem do yuzu: pasta de dados em %APPDATA%\eden,
# NAND emulada em nand/, e o save do usuario sob nand/user/save/, agrupado numa pasta por
# Title ID de 16 hexadecimais, depois do indice do espaco de save e do perfil de usuario.
param([Parameter(Mandatory=$true)][string]$Root)

$ErrorActionPreference = 'Stop'

# Marcadores da assinatura de pasta.
foreach ($sub in 'nand', 'config', 'keys', 'sdmc') {
    New-Item -ItemType Directory -Force -Path "$Root\$sub" | Out-Null
}

$perfil = "$Root\nand\user\save\0000000000000000\ABCDEF0123456789ABCDEF0123456789"

# Dois jogos. O indice do espaco de save (0000000000000000) tem a MESMA forma do Title ID e
# vem antes dele no caminho: se ele roubasse a identidade, os dois jogos virariam um so.
foreach ($tid in '0100000000010000', '0100000000020000') {
    New-Item -ItemType Directory -Force -Path "$perfil\$tid\meta" | Out-Null
    [IO.File]::WriteAllBytes("$perfil\$tid\progress.dat", [byte[]](1..64))
    [IO.File]::WriteAllBytes("$perfil\$tid\meta\header.bin", [byte[]](1..16))
}

# Ruido que NAO e save do usuario.
[IO.File]::WriteAllBytes("$Root\keys\prod.keys", [byte[]](1..32))
Set-Content -Path "$Root\config\qt-config.ini" -Value "[UI]" -NoNewline

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
