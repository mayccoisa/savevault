//! Tratamento de texto comum aos formatos de save de console.
//!
//! Vive aqui, e não dentro de um formato, porque o problema é o mesmo em todos: o console é
//! japonês, o título é escrito pelo jogo, e a forma como ele chega ao disco não é a forma como se
//! escreve num nome de pasta.

/// Converte as formas de largura inteira para ASCII comum.
///
/// Jogos gravam o título em largura inteira com frequência (`ＦＦ９` em vez de `FF9`), porque o
/// cartão é lido por um menu japonês. É mapeamento canônico do Unicode, não adivinhação.
pub fn normalize_width(text: &str) -> String {
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

/// Decodifica Shift-JIS até o primeiro zero, já normalizado.
///
/// Devolve `None` quando os bytes não são Shift-JIS válido ou quando não sobra texto. Recusar é
/// de propósito: um título em mojibake é pior que título nenhum, porque parece informação.
pub fn decode_shift_jis(raw: &[u8]) -> Option<String> {
    let raw = match raw.iter().position(|byte| *byte == 0) {
        Some(end) => &raw[..end],
        None => raw,
    };

    let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(raw);
    if had_errors {
        return None;
    }

    let text = normalize_width(decoded.trim());
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_full_width_to_ascii() {
        assert_eq!("FF9", normalize_width("ＦＦ９"));
        assert_eq!("A B", normalize_width("Ａ\u{3000}Ｂ"));
    }

    #[test]
    fn keeps_japanese_text_alone() {
        assert_eq!("ペプシマン", normalize_width("ペプシマン"));
    }

    #[test]
    fn decodes_until_the_terminator() {
        let mut raw = encoding_rs::SHIFT_JIS.encode("GOD OF WAR").0.into_owned();
        raw.extend_from_slice(&[0, b'X', b'Y']);

        assert_eq!(Some("GOD OF WAR".to_string()), decode_shift_jis(&raw));
    }

    #[test]
    fn refuses_what_is_not_shift_jis() {
        assert_eq!(None, decode_shift_jis(&[0x81, 0xFF, 0xFE]));
        assert_eq!(None, decode_shift_jis(&[]));
        assert_eq!(None, decode_shift_jis(b"   "));
    }
}
