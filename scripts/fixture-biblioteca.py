"""Cria um cofre sintético isolado para validar a biblioteca, sem saves reais."""
from pathlib import Path
import hashlib
import json
import sys

base = Path(sys.argv[1]).resolve()
if base.exists() and any(base.iterdir()):
    raise SystemExit("Use uma pasta vazia para o cofre sintético.")
base.mkdir(parents=True, exist_ok=True)
def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')

games = [('Hades', 1145360, 3), ('Hollow Knight', 367520, 4), ('Stardew Valley', 413150, 2), ('ELDEN RING', 1245620, 3),
         ('Jogo de teste com um título muito longo para verificar o comportamento da biblioteca', None, 1)]
manifest = {}
custom_games = []
for index, (title, steam_id, count) in enumerate(games):
    live = base / 'saves' / f'jogo-{index}.sav'
    live.parent.mkdir(exist_ok=True)
    live.write_text('progresso sintético atual', encoding='utf-8')
    folder = base / 'vault' / f'jogo-{index}'
    backups = []
    for version in range(count):
        content = f'progresso sintético {index} versão {version}'.encode()
        backup_name = f'backup-202610{10 + version:02}T120000Z'
        saved = folder / backup_name / 'drive-C' / live.as_posix()[3:]
        saved.parent.mkdir(parents=True, exist_ok=True)
        saved.write_bytes(content)
        backups.append({'name': backup_name, 'when': f'2026-10-{10 + version:02}T12:00:00Z', 'os': 'windows',
                        'files': {live.as_posix(): {'hash': hashlib.sha1(content).hexdigest(), 'size': len(content)}},
                        'comment': 'Dados sintéticos para validação visual'})
    write_json(folder/'mapping.yaml', {'name': title, 'drives': {'drive-C': 'C:'}, 'backups': backups})
    manifest[title] = {'files': {live.as_posix(): {}}, 'steam': {'id': steam_id} if steam_id else {}}
    custom_games.append({'name': title, 'files': [live.as_posix()]})

write_json(base/'manifest.yaml', manifest)
write_json(base/'config.yaml', {'language': 'pt-BR', 'theme': 'dark', 'release': {'check': False},
    'backup': {'path': (base/'vault').as_posix()}, 'restore': {'path': (base/'vault').as_posix()},
    'roots': [], 'customGames': custom_games})
print(base)
