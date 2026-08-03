//! Leitura do formato de memory card do PlayStation 1.
//!
//! Um memory card do PS1 é uma imagem raw de 131.072 bytes, sem cabeçalho, que emuladores
//! gravam como `.mcd` (DuckStation), `.mcr` (ePSXe) e afins. O formato é público e documentado,
//! e é por isso que dá para descobrir o nome do jogo sem depender de nenhuma base externa:
//!
//! * 16 blocos de 8.192 bytes, cada bloco com 64 frames de 128 bytes.
//! * O bloco 0 é o diretório. Os frames 1 a 15 descrevem os blocos de save 1 a 15:
//!   estado em `0x00..0x04` (`0x51` = em uso, primeiro bloco do save), e nome do arquivo em
//!   `0x0A..0x1F`, ASCII terminado em `0x00`. O nome carrega o código do jogo,
//!   por exemplo `BASLUS-01251FF9-01`.
//! * O primeiro frame de cada bloco de save é o "title frame": assinatura `SC` em `0x00..0x02`
//!   e o **título escrito pelo próprio jogo** em `0x04..0x44`, 64 bytes em Shift-JIS.
//!
//! Regra da casa aplicada em código: nada aqui inventa. Byte fora do formato, título que não
//! decodifica ou arquivo de tamanho errado resultam em ausência de informação, nunca em palpite.

/// Tamanho exato de uma imagem de memory card do PS1.
pub const CARD_SIZE: usize = 131_072;

const FRAME_SIZE: usize = 128;
const BLOCK_SIZE: usize = 8_192;
/// Blocos de save, isto é, todos menos o bloco 0 de diretório.
const SAVE_BLOCKS: usize = 15;

/// Estado do bloco: em uso, e é o primeiro (ou único) bloco do save.
const STATE_IN_USE_FIRST: u32 = 0x0000_0051;

const DIR_FILENAME: std::ops::Range<usize> = 0x0A..0x1F;
const TITLE_MAGIC: std::ops::Range<usize> = 0x00..0x02;
const TITLE_TEXT: std::ops::Range<usize> = 0x04..0x44;

/// Um save encontrado no cartão.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CardEntry {
    /// Bloco de save, de 1 a 15.
    pub slot: usize,
    /// Código de mídia do jogo, por exemplo `SLUS-01251`. É a identidade estável.
    pub serial: String,
    /// Título escrito pelo próprio jogo. `None` quando o cartão não traz ou não decodifica.
    pub title: Option<String>,
}

/// Lê os saves de uma imagem de memory card.
///
/// Devolve vazio quando os bytes não são uma imagem de cartão. Um save que ocupa vários blocos
/// aparece uma única vez, porque só o primeiro bloco é considerado.
pub fn read_entries(bytes: &[u8]) -> Vec<CardEntry> {
    if bytes.len() != CARD_SIZE {
        return vec![];
    }

    (1..=SAVE_BLOCKS)
        .filter_map(|slot| {
            let dir = bytes.get(FRAME_SIZE * slot..FRAME_SIZE * (slot + 1))?;
            if read_u32(dir)? != STATE_IN_USE_FIRST {
                return None;
            }
            let serial = serial_from_filename(dir.get(DIR_FILENAME)?)?;
            let title = bytes
                .get(BLOCK_SIZE * slot..BLOCK_SIZE * slot + FRAME_SIZE)
                .and_then(read_title);
            Some(CardEntry { slot, serial, title })
        })
        .collect()
}

fn read_u32(frame: &[u8]) -> Option<u32> {
    let raw: [u8; 4] = frame.get(0x00..0x04)?.try_into().ok()?;
    Some(u32::from_le_bytes(raw))
}

/// Extrai o código de mídia do nome do arquivo no diretório.
///
/// O nome segue `<região><código do jogo><identificador do jogo>`, por exemplo
/// `BASLUS-01251FF9-01`.
fn serial_from_filename(raw: &[u8]) -> Option<String> {
    let name: String = raw
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as char)
        .collect();
    media_code_in(&name)
}

/// Procura um código de mídia do PlayStation (`SLUS-00067`) dentro de um texto.
///
/// Procurado por forma (quatro letras, hífen, dígitos) e não por posição fixa, porque a posição
/// varia: no nome de arquivo do diretório do memory card ele vem depois do código de região
/// (`BASLUS-00067SOTN`), e no nome de um estado salvo vem no começo.
pub fn media_code_in(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();

    for start in 0..chars.len() {
        let Some(letters) = chars.get(start..start + 4) else {
            break;
        };
        if !letters.iter().all(|c| c.is_ascii_alphabetic()) || chars.get(start + 4) != Some(&'-') {
            continue;
        }
        let digits: String = chars[start + 5..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if (3..=5).contains(&digits.len()) {
            let letters: String = letters.iter().collect();
            return Some(format!("{}-{}", letters.to_ascii_uppercase(), digits));
        }
    }

    None
}

/// Lê o título do title frame de um bloco de save.
fn read_title(frame: &[u8]) -> Option<String> {
    if frame.get(TITLE_MAGIC)? != b"SC" {
        return None;
    }

    let raw = frame.get(TITLE_TEXT)?;
    // O título é preenchido com zeros até 64 bytes. Cortar antes de decodificar, para o
    // decodificador não ver o preenchimento como caractere.
    let raw = match raw.iter().position(|byte| *byte == 0) {
        Some(end) => &raw[..end],
        None => raw,
    };

    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(raw);
    if had_errors {
        return None;
    }

    let title = normalize_width(decoded.trim());
    (!title.is_empty()).then_some(title)
}

/// Converte as formas de largura inteira para ASCII comum.
///
/// Jogos de PS1 gravam o título em largura inteira com frequência (`ＦＦ９` em vez de `FF9`),
/// porque o cartão é lido por um menu japonês. É mapeamento canônico do Unicode, não adivinhação.
fn normalize_width(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{3000}' => ' ',
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFF01 + 0x21).unwrap_or(c),
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// Constrói uma imagem de cartão válida, para os testes exercitarem o formato real
    /// sem depender de save de ninguém.
    struct CardBuilder {
        bytes: Vec<u8>,
    }

    impl CardBuilder {
        fn formatted() -> Self {
            let mut bytes = vec![0u8; CARD_SIZE];
            // Todos os blocos livres e formatados.
            for slot in 1..=SAVE_BLOCKS {
                let at = FRAME_SIZE * slot;
                bytes[at..at + 4].copy_from_slice(&0x0000_00A0u32.to_le_bytes());
            }
            Self { bytes }
        }

        fn state(mut self, slot: usize, state: u32) -> Self {
            let at = FRAME_SIZE * slot;
            self.bytes[at..at + 4].copy_from_slice(&state.to_le_bytes());
            self
        }

        fn filename(mut self, slot: usize, name: &str) -> Self {
            let at = FRAME_SIZE * slot + DIR_FILENAME.start;
            let raw = name.as_bytes();
            self.bytes[at..at + raw.len()].copy_from_slice(raw);
            self
        }

        /// Grava o title frame com a assinatura correta e o título em Shift-JIS.
        fn title(mut self, slot: usize, title: &str) -> Self {
            let at = BLOCK_SIZE * slot;
            self.bytes[at..at + 2].copy_from_slice(b"SC");
            self.bytes[at + 0x02] = 0x11; // ícone de um quadro
            self.bytes[at + 0x03] = 0x01; // um bloco
            let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(title);
            let at = at + TITLE_TEXT.start;
            self.bytes[at..at + encoded.len()].copy_from_slice(&encoded);
            self
        }

        /// Title frame sem a assinatura `SC`.
        fn title_without_magic(mut self, slot: usize, title: &str) -> Self {
            let at = BLOCK_SIZE * slot + TITLE_TEXT.start;
            let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(title);
            self.bytes[at..at + encoded.len()].copy_from_slice(&encoded);
            self
        }

        /// Um save completo: diretório em uso, nome e título.
        fn save(self, slot: usize, name: &str, title: &str) -> Self {
            self.state(slot, STATE_IN_USE_FIRST).filename(slot, name).title(slot, title)
        }

        fn build(self) -> Vec<u8> {
            self.bytes
        }
    }

    #[test]
    fn reads_nothing_from_a_formatted_empty_card() {
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&CardBuilder::formatted().build()));
    }

    #[test]
    fn reads_serial_and_title_of_a_single_save() {
        let card = CardBuilder::formatted()
            .save(1, "BASLUS-00067SOTN", "CASTLEVANIA SOTN")
            .build();

        assert_eq!(
            vec![CardEntry {
                slot: 1,
                serial: "SLUS-00067".to_string(),
                title: Some("CASTLEVANIA SOTN".to_string()),
            }],
            read_entries(&card)
        );
    }

    #[test]
    fn reads_every_game_of_a_shared_card() {
        let card = CardBuilder::formatted()
            .save(1, "BASLUS-00067SOTN", "CASTLEVANIA SOTN")
            .save(2, "BASLUS-00594FF7", "FF7-01")
            .save(7, "BASCES-01438MGS", "METAL GEAR SOLID")
            .build();

        assert_eq!(
            vec![
                CardEntry {
                    slot: 1,
                    serial: "SLUS-00067".to_string(),
                    title: Some("CASTLEVANIA SOTN".to_string()),
                },
                CardEntry {
                    slot: 2,
                    serial: "SLUS-00594".to_string(),
                    title: Some("FF7-01".to_string()),
                },
                CardEntry {
                    slot: 7,
                    serial: "SCES-01438".to_string(),
                    title: Some("METAL GEAR SOLID".to_string()),
                },
            ],
            read_entries(&card)
        );
    }

    /// Um save de vários blocos tem um frame `0x51` seguido de `0x52`/`0x53`. Só o primeiro
    /// descreve o save, então ele não pode ser contado mais de uma vez.
    #[test]
    fn a_save_spanning_blocks_is_counted_once() {
        let card = CardBuilder::formatted()
            .save(1, "BASLUS-00662GRAN", "GRAN TURISMO")
            .state(2, 0x0000_0052)
            .state(3, 0x0000_0053)
            .build();

        let found = read_entries(&card);
        assert_eq!(1, found.len());
        assert_eq!("SLUS-00662", found[0].serial);
    }

    #[test]
    fn reads_a_japanese_title() {
        let card = CardBuilder::formatted()
            .save(1, "BASLPS-00700SAGA", "サガ フロンティア")
            .build();

        assert_eq!(
            vec![CardEntry {
                slot: 1,
                serial: "SLPS-00700".to_string(),
                title: Some("サガ フロンティア".to_string()),
            }],
            read_entries(&card)
        );
    }

    /// Jogos gravam o título em largura inteira com frequência. O usuário quer ler `FF9`.
    #[test]
    fn normalizes_full_width_titles_to_ascii() {
        let card = CardBuilder::formatted()
            .save(1, "BASLUS-01251FF9", "ＦＦ９　ＤＩＳＣ１")
            .build();

        assert_eq!(Some("FF9 DISC1".to_string()), read_entries(&card)[0].title);
    }

    /// Sem a assinatura o frame não é um title frame, e inventar título ali seria adivinhar.
    /// O jogo continua aparecendo, pelo serial.
    #[test]
    fn a_title_frame_without_magic_yields_no_title_but_keeps_the_game() {
        let card = CardBuilder::formatted()
            .state(1, STATE_IN_USE_FIRST)
            .filename(1, "BASLUS-00067SOTN")
            .title_without_magic(1, "CASTLEVANIA SOTN")
            .build();

        assert_eq!(
            vec![CardEntry {
                slot: 1,
                serial: "SLUS-00067".to_string(),
                title: None,
            }],
            read_entries(&card)
        );
    }

    #[test]
    fn ignores_a_save_whose_filename_has_no_media_code() {
        let card = CardBuilder::formatted().save(1, "MY-OWN-BACKUP", "WHATEVER").build();

        assert_eq!(Vec::<CardEntry>::new(), read_entries(&card));
    }

    #[test]
    fn reads_nothing_from_a_file_of_the_wrong_size() {
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&[]));
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0u8; 1024]));
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0u8; CARD_SIZE - 1]));
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0u8; CARD_SIZE + 1]));
    }

    /// Um cartão de lixo aleatório não pode gerar jogo nem pânico.
    #[test]
    fn reads_nothing_from_garbage_of_the_right_size() {
        let garbage: Vec<u8> = (0..CARD_SIZE).map(|i| (i % 251) as u8).collect();
        let _ = read_entries(&garbage);
    }

    #[test]
    fn extracts_the_media_code_from_real_world_filename_shapes() {
        let cases = [
            ("BASLUS-00067SOTN", Some("SLUS-00067")),
            ("BESCES-01438MGS1", Some("SCES-01438")),
            ("BISLPS-00700SAGA", Some("SLPS-00700")),
            ("BASLUS-94163DRACULA", Some("SLUS-94163")),
            ("baslus-00594ff7", Some("SLUS-00594")),
            ("BASCPS-45231", Some("SCPS-45231")),
            ("SLUS-00067", Some("SLUS-00067")),
            ("BA", None),
            ("", None),
            ("BASLUS-00", None),
            ("NOTASERIALATALL", None),
        ];

        for (filename, expected) in cases {
            assert_eq!(
                expected.map(|x| x.to_string()),
                serial_from_filename(filename.as_bytes()),
                "nome: {filename}"
            );
        }
    }
}
