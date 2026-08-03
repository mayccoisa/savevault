//! Leitura do `PARAM.SFO`, o arquivo de metadado que a Sony usa no PSP, no PS3 e no PS4.
//!
//! Serve para o mesmo fim que [`super::psx_card`] serve ao PS1: descobrir o **nome do jogo** a
//! partir do próprio save, sem base externa e sem internet. A diferença é que aqui o nome está
//! num arquivo declarado ao lado do save, e não escondido dentro de uma imagem de cartão.
//!
//! Formato, retirado do código do PPSSPP (`Core/ELF/ParamSFO.cpp`), que é aberto:
//!
//! ```text
//! cabeçalho, 20 bytes, tudo little-endian:
//!   0x00  u32  mágica, 0x46535000, que é "\0PSF"
//!   0x04  u32  versão, normalmente 0x00000101
//!   0x08  u32  início da tabela de chaves
//!   0x0C  u32  início da tabela de dados
//!   0x10  u32  quantidade de entradas
//!
//! cada entrada do índice, 16 bytes:
//!   0x00  u16  deslocamento da chave, relativo à tabela de chaves
//!   0x02  u16  formato do dado
//!   0x04  u32  bytes usados
//!   0x08  u32  bytes reservados
//!   0x0C  u32  deslocamento do dado, relativo à tabela de dados
//! ```
//!
//! Formatos: `0x0204` é texto UTF-8 terminado em zero, `0x0004` é texto UTF-8 sem terminador, e
//! `0x0404` é inteiro sem sinal. Só o texto interessa aqui.

/// Mágica do cabeçalho, `"\0PSF"` lido como inteiro little-endian.
const MAGIC: u32 = 0x4653_5000;
const HEADER_SIZE: usize = 20;
const INDEX_ENTRY_SIZE: usize = 16;

/// Texto UTF-8, terminado em zero.
const FMT_UTF8: u16 = 0x0204;
/// Texto UTF-8 sem terminador.
const FMT_UTF8_RAW: u16 = 0x0004;

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(raw))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(at..at + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(raw))
}

/// Lê um texto terminado em zero, ou o pedaço inteiro quando não há terminador.
fn text_at(bytes: &[u8], at: usize, len: usize) -> Option<String> {
    let raw = bytes.get(at..at + len)?;
    let raw = match raw.iter().position(|byte| *byte == 0) {
        Some(end) => &raw[..end],
        None => raw,
    };
    let text = std::str::from_utf8(raw).ok()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Lê o valor de texto de uma chave.
///
/// Devolve `None` quando o arquivo não é um SFO, está truncado, a chave não existe, ou o valor
/// não é texto. Nunca entra em pânico: isto lê arquivo do disco de outra pessoa, e um arquivo
/// corrompido tem que virar "não sei", não uma queda do programa.
pub fn string_value(bytes: &[u8], wanted: &str) -> Option<String> {
    if u32_at(bytes, 0)? != MAGIC {
        return None;
    }

    let key_table = u32_at(bytes, 0x08)? as usize;
    let data_table = u32_at(bytes, 0x0C)? as usize;
    let entries = u32_at(bytes, 0x10)? as usize;

    // Um cabeçalho mentiroso não pode virar uma varredura de milhões de entradas.
    let max_entries = bytes.len().saturating_sub(HEADER_SIZE) / INDEX_ENTRY_SIZE;
    let entries = entries.min(max_entries);

    for index in 0..entries {
        let at = HEADER_SIZE + index * INDEX_ENTRY_SIZE;

        let key_offset = u16_at(bytes, at)? as usize;
        let format = u16_at(bytes, at + 0x02)?;
        let len = u32_at(bytes, at + 0x04)? as usize;
        let data_offset = u32_at(bytes, at + 0x0C)? as usize;

        if format != FMT_UTF8 && format != FMT_UTF8_RAW {
            continue;
        }

        // A chave vai até o zero; o tamanho não está declarado em lugar nenhum.
        let key_start = key_table.checked_add(key_offset)?;
        let key_len = bytes.get(key_start..)?.iter().position(|byte| *byte == 0)?;
        let key = std::str::from_utf8(bytes.get(key_start..key_start + key_len)?).ok()?;

        if key != wanted {
            continue;
        }

        return text_at(bytes, data_table.checked_add(data_offset)?, len);
    }

    None
}

/// O nome do jogo, como o próprio save o declara.
pub fn title(bytes: &[u8]) -> Option<String> {
    string_value(bytes, "TITLE")
}

/// Um identificador de jogo da PlayStation em texto: quatro letras e cinco dígitos.
///
/// Serve ao PSP (`ULUS12345`) e ao PS3 (`BLES00932`, `NPEA00275`), que usam a mesma forma. O
/// nome da pasta traz sufixos que variam por jogo e por tipo de save (`ULUS1234501`,
/// `BLES00932-AUTO`), então o que vale é o **começo**.
pub fn title_id_prefix(name: &str) -> Option<String> {
    let chars: Vec<char> = name.chars().collect();
    let letters = chars.get(0..4)?;
    let digits = chars.get(4..9)?;

    (letters.iter().all(|c| c.is_ascii_alphabetic()) && digits.iter().all(|c| c.is_ascii_digit()))
        .then(|| format!("{}{}", letters.iter().collect::<String>().to_ascii_uppercase(), digits.iter().collect::<String>()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monta um SFO válido segundo o formato, para não depender de arquivo de save de ninguém.
    fn sfo(pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut keys: Vec<u8> = vec![];
        let mut data: Vec<u8> = vec![];
        let mut index: Vec<u8> = vec![];

        for (key, value) in pairs {
            let key_offset = keys.len() as u16;
            keys.extend_from_slice(key.as_bytes());
            keys.push(0);

            let data_offset = data.len() as u32;
            let mut value_bytes = value.as_bytes().to_vec();
            value_bytes.push(0);
            let len = value_bytes.len() as u32;
            data.extend_from_slice(&value_bytes);

            index.extend_from_slice(&key_offset.to_le_bytes());
            index.extend_from_slice(&FMT_UTF8.to_le_bytes());
            index.extend_from_slice(&len.to_le_bytes());
            index.extend_from_slice(&len.to_le_bytes());
            index.extend_from_slice(&data_offset.to_le_bytes());
        }

        let key_table = (HEADER_SIZE + index.len()) as u32;
        let data_table = key_table + keys.len() as u32;

        let mut out = vec![];
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&0x0000_0101u32.to_le_bytes());
        out.extend_from_slice(&key_table.to_le_bytes());
        out.extend_from_slice(&data_table.to_le_bytes());
        out.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&keys);
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn reads_the_title() {
        let bytes = sfo(&[
            ("CATEGORY", "MS"),
            ("TITLE", "MONSTER HUNTER FREEDOM UNITE"),
            ("SAVEDATA_DETAIL", "Village 5"),
        ]);

        assert_eq!(Some("MONSTER HUNTER FREEDOM UNITE".to_string()), title(&bytes));
    }

    #[test]
    fn reads_a_key_that_is_not_the_first_one() {
        let bytes = sfo(&[("CATEGORY", "MS"), ("SAVEDATA_DETAIL", "Slot 1")]);

        assert_eq!(Some("Slot 1".to_string()), string_value(&bytes, "SAVEDATA_DETAIL"));
        assert_eq!(None, title(&bytes));
    }

    #[test]
    fn a_missing_key_is_not_an_error() {
        let bytes = sfo(&[("CATEGORY", "MS")]);
        assert_eq!(None, title(&bytes));
    }

    /// Estes três são o motivo de a função não entrar em pânico: são arquivos reais que aparecem
    /// numa pasta de emulador, e nenhum deles pode derrubar a varredura.
    #[test]
    fn refuses_anything_that_is_not_an_sfo() {
        assert_eq!(None, title(b""));
        assert_eq!(None, title(b"nao e um sfo"));
        assert_eq!(None, title(&[0u8; 64]));
    }

    #[test]
    fn survives_a_truncated_file() {
        let bytes = sfo(&[("TITLE", "PERSONA 3 PORTABLE")]);

        for cut in 1..bytes.len() {
            // O que importa é não entrar em pânico; devolver `None` é resposta legítima.
            let _ = title(&bytes[..cut]);
        }
    }

    /// Um cabeçalho que declara mais entradas do que cabem no arquivo não pode virar uma
    /// varredura sem fim nem uma leitura fora do limite.
    #[test]
    fn survives_a_lying_entry_count() {
        let mut bytes = sfo(&[("TITLE", "GOD OF WAR")]);
        bytes[0x10..0x14].copy_from_slice(&u32::MAX.to_le_bytes());

        assert_eq!(Some("GOD OF WAR".to_string()), title(&bytes));
    }

    #[test]
    fn ignores_a_value_that_is_not_text() {
        let mut bytes = sfo(&[("TITLE", "TEKKEN 6")]);
        // Formato inteiro na primeira entrada do índice.
        bytes[HEADER_SIZE + 0x02..HEADER_SIZE + 0x04].copy_from_slice(&0x0404u16.to_le_bytes());

        assert_eq!(None, title(&bytes));
    }

    #[test]
    fn recognizes_playstation_title_ids_by_shape() {
        assert_eq!(Some("ULUS12345".to_string()), title_id_prefix("ULUS1234501"));
        assert_eq!(Some("BLES00932".to_string()), title_id_prefix("BLES00932-AUTO"));
        assert_eq!(Some("NPEA00275".to_string()), title_id_prefix("NPEA00275"));
        assert_eq!(Some("ULUS12345".to_string()), title_id_prefix("ulus12345"));
        assert_eq!(None, title_id_prefix("SAVEDATA"));
        assert_eq!(None, title_id_prefix("ULUS123"));
        assert_eq!(None, title_id_prefix("12345ULUS"));
    }
}
