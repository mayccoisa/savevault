//! Leitura do cabeçalho do estado salvo do DuckStation.
//!
//! Serve ao mesmo fim que [`super::psx_card`] e [`super::param_sfo`]: descobrir o **nome do jogo**
//! a partir do próprio save, sem base externa e sem internet. Aqui o nome está no cabeçalho do
//! arquivo, em claro, o que resolve o caso de quem só usa estado salvo e nunca gravou num memory
//! card — que, num backup real, era a maioria dos jogos.
//!
//! Formato, retirado de `src/core/save_state_version.h` do DuckStation, struct
//! `SAVE_STATE_HEADER`, tudo little-endian:
//!
//! ```text
//!   0x00  u32       mágica, 0x43435544, que é "DUCC"
//!   0x04  u32       versão
//!   0x08  char[128] título
//!   0x88  char[32]  serial
//!   0xA8  ...       deslocamentos da mídia, da captura de tela e dos dados
//! ```
//!
//! Ao contrário do PS1 e do PS2, este título **não vem do jogo**: o DuckStation o copia do próprio
//! banco de dados dele, já em UTF-8. Por isso não há Shift-JIS aqui.
//!
//! Nada precisa ser descomprimido. O estado em si pode estar em deflate, zstd ou xz, mas o
//! cabeçalho está sempre em claro no começo do arquivo, e é só ele que se lê.

/// Mágica do cabeçalho, `"DUCC"` lida como inteiro little-endian.
const MAGIC: u32 = 0x4343_5544;

/// A versão mais antiga que o próprio DuckStation aceita carregar
/// (`SAVE_STATE_MINIMUM_VERSION`). Abaixo dela o layout não foi verificado na fonte, então recusar
/// é a única resposta honesta.
const MINIMUM_VERSION: u32 = 42;

const TITLE_AT: usize = 0x08;
const TITLE_LEN: usize = 128;
const SERIAL_AT: usize = 0x88;
const SERIAL_LEN: usize = 32;

/// Quantos bytes do começo do arquivo bastam. O resto do estado salvo chega a 32 MB e não
/// interessa a ninguém aqui.
pub const HEADER_PREFIX: usize = SERIAL_AT + SERIAL_LEN;

/// O que o cabeçalho de um estado salvo declara sobre o jogo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateInfo {
    pub version: u32,
    pub serial: Option<String>,
    pub title: Option<String>,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(raw))
}

/// Lê um campo de texto de tamanho fixo.
///
/// Corta no primeiro zero, ou usa o campo inteiro quando não há nenhum. É o `Strnlen` que o
/// próprio DuckStation usa ao carregar, e não uma escolha nossa.
fn text_field(bytes: &[u8], at: usize, len: usize) -> Option<String> {
    let raw = bytes.get(at..at + len)?;
    let raw = match raw.iter().position(|byte| *byte == 0) {
        Some(end) => &raw[..end],
        None => raw,
    };
    let text = std::str::from_utf8(raw).ok()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Lê o que o cabeçalho declara, ou `None` se isto não for um estado salvo do DuckStation.
///
/// Nunca entra em pânico: isto lê arquivo do disco de outra pessoa, e arquivo corrompido tem que
/// virar "não sei", não uma queda do programa. Título ilegível não derruba o serial, nem o
/// contrário: são dois fatos independentes, e perder um não é motivo para jogar fora o outro.
pub fn read_header(bytes: &[u8]) -> Option<StateInfo> {
    if u32_at(bytes, 0)? != MAGIC {
        return None;
    }

    let version = u32_at(bytes, 0x04)?;
    if version < MINIMUM_VERSION {
        return None;
    }
    // Deliberadamente sem teto. O emulador recusa versão futura porque precisa desserializar o
    // estado da máquina inteiro; nós lemos 168 bytes que não mudaram em nenhuma versão que a
    // fonte cobre. Travar o teto faria o SaveVault parar de ler título a cada atualização do
    // DuckStation, sozinho, sem ninguém mexer em nada.

    Some(StateInfo {
        version,
        // O serial passa pela mesma normalização de forma do resto da base, para que
        // `SLUS-00774` vindo do cabeçalho e vindo do nome do arquivo sejam a mesma chave.
        serial: text_field(bytes, SERIAL_AT, SERIAL_LEN).and_then(|raw| super::psx_card::media_code_in(&raw)),
        title: text_field(bytes, TITLE_AT, TITLE_LEN),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monta um cabeçalho válido segundo o formato, para não depender de save de ninguém.
    fn header(version: u32, title: &[u8], serial: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER_PREFIX];
        bytes[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        bytes[4..8].copy_from_slice(&version.to_le_bytes());
        bytes[TITLE_AT..TITLE_AT + title.len().min(TITLE_LEN)].copy_from_slice(title);
        bytes[SERIAL_AT..SERIAL_AT + serial.len().min(SERIAL_LEN)].copy_from_slice(serial);
        bytes
    }

    #[test]
    fn reads_the_title_and_serial() {
        let info = read_header(&header(83, b"X-Men - Mutant Academy", b"SLUS-00774")).unwrap();

        assert_eq!(83, info.version);
        assert_eq!(Some("X-Men - Mutant Academy".to_string()), info.title);
        assert_eq!(Some("SLUS-00774".to_string()), info.serial);
    }

    /// O serial tem 32 bytes, não 64. Ler demais invadiria `media_path_length`, que é zero num
    /// estado de disco físico e vira lixo num estado de arquivo. Este teste trava o tamanho.
    #[test]
    fn the_serial_field_stops_before_the_next_field() {
        let mut bytes = header(83, b"Whatever", b"SLUS-00774");
        bytes.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());

        assert_eq!(Some("SLUS-00774".to_string()), read_header(&bytes).unwrap().serial);
    }

    /// O DuckStation lê com `Strnlen`: sem zero, vale o campo inteiro.
    #[test]
    fn a_title_that_fills_the_field_has_no_terminator() {
        let long = vec![b'A'; TITLE_LEN];

        assert_eq!(
            Some("A".repeat(TITLE_LEN)),
            read_header(&header(83, &long, b"SLUS-00774")).unwrap().title
        );
    }

    #[test]
    fn text_after_the_terminator_is_ignored() {
        let mut bytes = header(83, b"Real Title", b"SLUS-00774");
        bytes[TITLE_AT + 40] = b'X';

        assert_eq!(Some("Real Title".to_string()), read_header(&bytes).unwrap().title);
    }

    #[test]
    fn refuses_a_version_older_than_the_emulator_accepts() {
        assert_eq!(None, read_header(&header(41, b"Title", b"SLUS-00774")));
        assert!(read_header(&header(42, b"Title", b"SLUS-00774")).is_some());
    }

    /// Uma versão futura ainda é lida: o pedaço que lemos é estável, e recusá-la faria o app
    /// quebrar sozinho quando o usuário atualizasse o emulador.
    #[test]
    fn accepts_a_future_version() {
        assert!(read_header(&header(200, b"Title", b"SLUS-00774")).is_some());
    }

    #[test]
    fn refuses_a_file_that_is_not_a_duckstation_state() {
        assert_eq!(None, read_header(b"PK\x03\x04 this is a zip"));
        assert_eq!(None, read_header(&[]));
        assert_eq!(None, read_header(&[0u8; 8]));
    }

    /// Um título ilegível não pode levar o serial junto: sem serial o save vira "não
    /// identificado" e sai de perto do resto do progresso daquele jogo.
    #[test]
    fn an_unreadable_title_keeps_the_serial() {
        let info = read_header(&header(83, &[0xFF, 0xFE, 0xFD], b"SLUS-00774")).unwrap();

        assert_eq!(None, info.title);
        assert_eq!(Some("SLUS-00774".to_string()), info.serial);
    }

    #[test]
    fn a_missing_serial_keeps_the_title() {
        let info = read_header(&header(83, b"Some Game", b"")).unwrap();

        assert_eq!(Some("Some Game".to_string()), info.title);
        assert_eq!(None, info.serial);
    }

    #[test]
    fn reads_a_title_with_non_ascii_characters() {
        let info = read_header(&header(83, "Ape Escape — Special".as_bytes(), b"SCUS-94423")).unwrap();

        assert_eq!(Some("Ape Escape — Special".to_string()), info.title);
    }

    /// Arquivo cortado no meio, em todo tamanho possível, sem pânico.
    #[test]
    fn survives_a_truncated_file() {
        let whole = header(83, b"X-Men - Mutant Academy", b"SLUS-00774");

        for cut in 0..whole.len() {
            let _ = read_header(&whole[..cut]);
        }
    }
}
