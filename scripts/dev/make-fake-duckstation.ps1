# Monta uma pasta de dados de DuckStation falsa, com um memory card VALIDO segundo o formato
# documentado do PS1, para provar backup e restauracao sem instalar emulador nenhum.
param([Parameter(Mandatory=$true)][string]$Root)

$ErrorActionPreference = 'Stop'

function New-PsxCard {
    param([hashtable[]]$Games)  # cada item: @{ Filename = 'BASLUS-00067SOTN'; Title = 'CASTLEVANIA SOTN' }

    $bytes = New-Object byte[] 131072

    # Bloco 0, frames 1..15: todos livres e formatados (0xA0).
    for ($slot = 1; $slot -le 15; $slot++) {
        $at = 128 * $slot
        [BitConverter]::GetBytes([uint32]0xA0).CopyTo($bytes, $at)
    }

    $slot = 1
    foreach ($g in $Games) {
        $at = 128 * $slot
        # 0x00..0x04 = estado: em uso, primeiro bloco.
        [BitConverter]::GetBytes([uint32]0x51).CopyTo($bytes, $at)
        # 0x0A..0x1F = nome do arquivo, ASCII terminado em 0x00.
        $name = [Text.Encoding]::ASCII.GetBytes($g.Filename)
        $name.CopyTo($bytes, $at + 0x0A)

        # Title frame do bloco de save: assinatura "SC" e titulo em Shift-JIS.
        $blk = 8192 * $slot
        [Text.Encoding]::ASCII.GetBytes('SC').CopyTo($bytes, $blk)
        $bytes[$blk + 0x02] = 0x11
        $bytes[$blk + 0x03] = 0x01
        $sjis = [Text.Encoding]::GetEncoding(932).GetBytes($g.Title)
        $sjis.CopyTo($bytes, $blk + 0x04)

        $slot++
    }

    return $bytes
}

New-Item -ItemType Directory -Force -Path "$Root\memcards" | Out-Null
New-Item -ItemType Directory -Force -Path "$Root\savestates" | Out-Null
New-Item -ItemType Directory -Force -Path "$Root\bios" | Out-Null
New-Item -ItemType Directory -Force -Path "$Root\covers" | Out-Null

# Marcadores da assinatura de pasta.
Set-Content -Path "$Root\settings.ini" -Value "[Main]`nSettingsVersion = 3" -NoNewline

# Um cartao por jogo, nomeado pelo serial.
[IO.File]::WriteAllBytes("$Root\memcards\SLUS-00067_1.mcd",
    (New-PsxCard -Games @(@{ Filename = 'BASLUS-00067SOTN'; Title = 'CASTLEVANIA SOTN' })))

# Um cartao nomeado pelo TITULO do jogo, para provar que a identidade vem de dentro do arquivo.
[IO.File]::WriteAllBytes("$Root\memcards\Final Fantasy VII_1.mcd",
    (New-PsxCard -Games @(@{ Filename = 'BASLUS-00594FF7'; Title = 'FF7-01' })))

# Um cartao compartilhado, com dois jogos dentro do mesmo arquivo indivisivel.
[IO.File]::WriteAllBytes("$Root\memcards\shared_card_1.mcd",
    (New-PsxCard -Games @(
        @{ Filename = 'BASCES-01438MGS'; Title = 'METAL GEAR SOLID' },
        @{ Filename = 'BASLPS-00700SAGA'; Title = 'サガ フロンティア' }
    )))

# Um estado salvo, identificado pelo nome do arquivo.
[IO.File]::WriteAllBytes("$Root\savestates\SLUS-00067_1.sav", [byte[]](1..64))

# Ruido que NAO deve virar save.
[IO.File]::WriteAllBytes("$Root\bios\scph1001.bin", [byte[]](1..32))
Set-Content -Path "$Root\covers\SLUS-00067.jpg" -Value "nao e save" -NoNewline
Set-Content -Path "$Root\memcards\readme.txt" -Value "nao e save" -NoNewline

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
