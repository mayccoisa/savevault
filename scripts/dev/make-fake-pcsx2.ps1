# Monta uma pasta de dados de PCSX2 falsa, para provar backup e restauracao sem instalar
# emulador nenhum. O layout segue o codigo do proprio PCSX2 (EmuFolders, em Pcsx2Config.cpp):
# subpastas bios/memcards/sstates/cache/covers, configuracao em inis/PCSX2.ini, e o nome do
# estado salvo no formato "{serial} ({crc:08X}).{slot:02}.p2s".
param([Parameter(Mandatory=$true)][string]$Root)

$ErrorActionPreference = 'Stop'

foreach ($sub in 'memcards', 'sstates', 'inis', 'bios', 'covers', 'cache') {
    New-Item -ItemType Directory -Force -Path "$Root\$sub" | Out-Null
}

# Marcador da assinatura de pasta: e a pasta `inis` que distingue o PCSX2 do DuckStation,
# porque `memcards` existe nos dois e `portable.txt` tambem.
Set-Content -Path "$Root\inis\PCSX2.ini" -Value "[UI]`nSettingsVersion = 1" -NoNewline

# Os dois cartoes padrao. O cartao do PS2 e um sistema de arquivos interno e o nome padrao nao
# carrega serial: estes devem aparecer como identidade opaca, NAO desaparecer.
[IO.File]::WriteAllBytes("$Root\memcards\Mcd001.ps2", [byte[]](1..64))
[IO.File]::WriteAllBytes("$Root\memcards\Mcd002.ps2", [byte[]](1..64))

# Um cartao nomeado pelo serial, que ganha identificacao de graca.
[IO.File]::WriteAllBytes("$Root\memcards\SLUS-20062.ps2", [byte[]](1..64))

# Estados salvos, identificados pelo serial no nome do arquivo.
[IO.File]::WriteAllBytes("$Root\sstates\SLUS-20062 (7ACF7E77).00.p2s", [byte[]](1..64))
[IO.File]::WriteAllBytes("$Root\sstates\SCES-50916 (0D6F0F0F).01.p2s", [byte[]](1..64))

# Ruido que NAO deve virar save: a copia de seguranca do estado e do emulador, e o resto
# nao e progresso do usuario.
[IO.File]::WriteAllBytes("$Root\sstates\SLUS-20062 (7ACF7E77).00.p2s.backup", [byte[]](1..64))
[IO.File]::WriteAllBytes("$Root\bios\scph39001.bin", [byte[]](1..32))
Set-Content -Path "$Root\covers\SLUS-20062.jpg" -Value "nao e save" -NoNewline
Set-Content -Path "$Root\memcards\readme.txt" -Value "nao e save" -NoNewline

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
