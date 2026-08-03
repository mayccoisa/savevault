# Monta uma pasta de dados de Xenia falsa, para provar backup e restauracao sem instalar
# emulador nenhum. Fonte do layout: codigo do proprio Xenia.
#   - raiz: pasta do executavel com `portable.txt` ao lado, senao <pasta do usuario>\Xenia
#   - conteudo: content\<title id>\<tipo de conteudo>\<nome>\  (ResolvePackageRoot)
#   - tipo 00000001 = save (XContentType::kSavedGame); 00004000 = jogo instalado
param(
    [Parameter(Mandatory=$true)][string]$Root,
    [switch]$Empty
)

$ErrorActionPreference = 'Stop'

foreach ($sub in 'content', 'cache') {
    New-Item -ItemType Directory -Force -Path "$Root\$sub" | Out-Null
}
Set-Content -Path "$Root\xenia.config.toml" -Value "[General]" -NoNewline

if (-not $Empty) {
    foreach ($tid in '4D5307E6', '584108A9') {
        New-Item -ItemType Directory -Force -Path "$Root\content\$tid\00000001\savegame" | Out-Null
        [IO.File]::WriteAllBytes("$Root\content\$tid\00000001\savegame\save.bin", [byte[]](1..64))
    }

    # O jogo INSTALADO mora na mesma pasta de titulo que o save, e nao pode entrar no backup.
    New-Item -ItemType Directory -Force -Path "$Root\content\4D5307E6\00004000\000D0000" | Out-Null
    [IO.File]::WriteAllBytes("$Root\content\4D5307E6\00004000\000D0000\jogo.xex", (New-Object byte[] 4096))
}

Write-Output "pasta falsa criada em $Root"
Get-ChildItem -Recurse $Root | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    "  {0}  ({1} bytes)" -f $_.FullName.Substring($Root.Length + 1), $_.Length
}
