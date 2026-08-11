//! Leitura do sistema de arquivos do memory card do PlayStation 2.
//!
//! O cartão do PS2 não é o do PS1. Onde o PS1 tem um diretório fixo de 15 blocos, o PS2 tem um
//! sistema de arquivos de verdade: superbloco, FAT indireta, e pastas com arquivos dentro. Cada
//! jogo tem **uma pasta**, cujo nome carrega o serial (`BASLUS-21004MAYC`), e dentro dela um
//! `icon.sys`, que é onde o jogo grava o nome que aparece no menu do console.
//!
//! Fontes, todas código aberto:
//!
//! * `pcsx2/SIO/Memcard/MemoryCardFolder.h` do PCSX2 — structs `superblock` e
//!   `MemoryCardFileEntry`, as flags de modo e a geometria (`PageSize`, `EccSize`, `PageSizeRaw`).
//! * `pcsx2/SIO/Memcard/MemoryCardFile.cpp` — `GetMemoryCardFileTypeFromSize`, que é como o
//!   próprio PCSX2 decide se um cartão tem ECC: **pelo tamanho do arquivo**.
//! * `ps2sdk`, `ee/rpc/memorycard/include/libmc.h` — struct `mcIcon`, que é o `icon.sys`.
//!   Note que esta última **não é do PCSX2**: o emulador trata o cartão como bloco opaco e nunca
//!   precisa do título. O ps2sdk é o SDK que define a struct que os jogos e o menu do console
//!   usam, e é a definição normativa disponível.
//!
//! A armadilha que o formato esconde: um cluster lógico de 1.024 bytes **não é contíguo no
//! arquivo** quando o cartão tem ECC. São dois pedaços de 512 separados por 16 bytes de correção
//! de erro, e é preciso costurá-los. Ler 1.024 bytes seguidos daria dados deslocados a partir do
//! segundo pedaço, sem erro nenhum aparente.

use super::text;

const MAGIC: &[u8] = b"Sony PS2 Memory Card Format";

/// Bytes úteis de uma página. O resto do passo, quando existe, é ECC.
const PAGE_DATA: usize = 512;
/// Página com ECC (`PageSizeRaw` do PCSX2) e sem ECC.
const PAGE_STRIDE_ECC: usize = 528;
const PAGE_STRIDE_PLAIN: usize = PAGE_DATA;

const DIR_ENTRY_SIZE: usize = 512;
/// Entradas de u32 numa FAT de um cluster (1.024 bytes).
const FAT_ENTRIES_PER_CLUSTER: usize = 256;

const MODE_FILE: u32 = 0x0010;
const MODE_DIRECTORY: u32 = 0x0020;
const MODE_USED: u32 = 0x8000;

const FAT_IN_USE: u32 = 0x8000_0000;
const FAT_END: u32 = 0xFFFF_FFFF;

/// Teto de clusters percorridos numa cadeia. Um cartão corrompido pode ter FAT cíclica, e uma
/// varredura de backup não pode travar por causa do arquivo de outra pessoa.
const MAX_CHAIN: usize = 8_192;

/// O arquivo de metadado que o jogo grava com o nome que aparece no menu do console.
const ICON_FILE: &str = "icon.sys";
const ICON_MAGIC: &[u8] = b"PS2D";
const ICON_NL_OFFSET_AT: usize = 0x06;
const ICON_TITLE_AT: usize = 0xC0;
const ICON_TITLE_LEN: usize = 68;

/// Um jogo encontrado no cartão.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CardEntry {
    /// Nome da pasta no cartão, por exemplo `BASLUS-21004MAYC`.
    pub folder: String,
    /// Código de mídia do jogo, por exemplo `SLUS-21004`. É a identidade estável.
    pub serial: String,
    /// Nome escrito pelo próprio jogo. `None` quando o cartão não traz ou não decodifica.
    pub title: Option<String>,
}

/// Geometria do cartão, deduzida do tamanho do arquivo.
struct Card<'a> {
    bytes: &'a [u8],
    stride: usize,
    alloc_offset: u32,
    clusters: u32,
    ifc: Vec<u32>,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(at..at + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(raw))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(raw))
}

/// Quantos bytes tem cada página, deduzido do tamanho do arquivo.
///
/// É o método do próprio PCSX2 (`GetMemoryCardFileTypeFromSize`), não uma heurística nossa: um
/// cartão de 8 MB tem 8.650.752 bytes com ECC e 8.388.608 sem. Tamanho fora da tabela devolve
/// `None`, porque adivinhar o passo de página desloca **todos** os deslocamentos seguintes e
/// produziria lixo com cara de dado.
fn stride_for(len: usize) -> Option<usize> {
    const RAW_MB: usize = 1024 * PAGE_STRIDE_ECC * 2;
    const PLAIN_MB: usize = 1024 * 1024;

    [8usize, 16, 32, 64].into_iter().find_map(|mb| {
        if len == mb * RAW_MB {
            Some(PAGE_STRIDE_ECC)
        } else if len == mb * PLAIN_MB {
            Some(PAGE_STRIDE_PLAIN)
        } else {
            None
        }
    })
}

impl<'a> Card<'a> {
    fn open(bytes: &'a [u8]) -> Option<Self> {
        let stride = stride_for(bytes.len())?;

        let superblock = bytes.get(0..PAGE_DATA)?;
        if !superblock.starts_with(MAGIC) {
            return None;
        }

        // Geometria fora do que a fonte cobre não é palpite nosso para resolver.
        if u16_at(superblock, 0x28)? != PAGE_DATA as u16 || u16_at(superblock, 0x2A)? != 2 {
            return None;
        }

        let ifc = (0..32)
            .filter_map(|index| u32_at(superblock, 0x50 + index * 4))
            .collect();

        Some(Self {
            bytes,
            stride,
            alloc_offset: u32_at(superblock, 0x34)?,
            clusters: u32_at(superblock, 0x30)?,
            ifc,
        })
    }

    fn page(&self, index: usize) -> Option<&'a [u8]> {
        let at = index.checked_mul(self.stride)?;
        self.bytes.get(at..at.checked_add(PAGE_DATA)?)
    }

    /// Um cluster absoluto, costurando as duas páginas que o compõem.
    fn cluster(&self, index: u32) -> Option<Vec<u8>> {
        let first = (index as usize).checked_mul(2)?;
        let mut out = Vec::with_capacity(PAGE_DATA * 2);
        out.extend_from_slice(self.page(first)?);
        out.extend_from_slice(self.page(first + 1)?);
        Some(out)
    }

    /// Um cluster da área de dados, cuja numeração é relativa ao `alloc_offset`.
    fn data_cluster(&self, index: u32) -> Option<Vec<u8>> {
        if index >= self.clusters {
            return None;
        }
        self.cluster(self.alloc_offset.checked_add(index)?)
    }

    /// O próximo cluster da cadeia, pela FAT indireta.
    fn next_in_chain(&self, index: u32) -> Option<u32> {
        let per = FAT_ENTRIES_PER_CLUSTER as u32;

        let indirect = self.cluster(*self.ifc.get((index / (per * per)) as usize)?)?;
        let fat_cluster = u32_at(&indirect, ((index / per) % per) as usize * 4)?;
        let fat = self.cluster(fat_cluster)?;
        let entry = u32_at(&fat, (index % per) as usize * 4)?;

        if entry == FAT_END || entry & FAT_IN_USE == 0 {
            return None;
        }
        Some(entry & !FAT_IN_USE)
    }

    /// Os clusters de dados de uma pasta ou arquivo, na ordem da cadeia.
    fn chain(&self, start: u32) -> Vec<u32> {
        let mut chain = vec![];
        let mut seen = std::collections::HashSet::new();
        let mut current = start;

        while chain.len() < MAX_CHAIN && seen.insert(current) {
            chain.push(current);
            match self.next_in_chain(current) {
                Some(next) => current = next,
                None => break,
            }
        }

        chain
    }

    /// As entradas de um diretório, seguindo a cadeia dele.
    fn dir_entries(&self, start: u32, count: u32) -> Vec<DirEntry> {
        let wanted = count as usize;
        let mut entries = vec![];

        for cluster in self.chain(start) {
            let Some(data) = self.data_cluster(cluster) else {
                break;
            };
            for slot in 0..data.len() / DIR_ENTRY_SIZE {
                if entries.len() >= wanted {
                    return entries;
                }
                if let Some(entry) = DirEntry::read(&data[slot * DIR_ENTRY_SIZE..]) {
                    entries.push(entry);
                }
            }
        }

        entries
    }

    /// O conteúdo de um arquivo, seguindo a cadeia dele.
    fn file_contents(&self, start: u32, len: u32) -> Vec<u8> {
        let mut out = Vec::new();

        for cluster in self.chain(start) {
            let Some(data) = self.data_cluster(cluster) else {
                break;
            };
            out.extend_from_slice(&data);
            if out.len() >= len as usize {
                break;
            }
        }

        out.truncate(len as usize);
        out
    }
}

/// Uma entrada de diretório do cartão.
struct DirEntry {
    mode: u32,
    length: u32,
    cluster: u32,
    name: String,
}

impl DirEntry {
    fn read(bytes: &[u8]) -> Option<Self> {
        let raw = bytes.get(0..DIR_ENTRY_SIZE)?;
        let name: String = raw
            .get(0x40..0x60)?
            .iter()
            .take_while(|byte| **byte != 0)
            .map(|byte| *byte as char)
            .collect();

        Some(Self {
            mode: u32_at(raw, 0x00)?,
            length: u32_at(raw, 0x04)?,
            cluster: u32_at(raw, 0x10)?,
            name,
        })
    }

    fn is_used(&self) -> bool {
        self.mode & MODE_USED != 0
    }

    /// Uma pasta de save de verdade. `.` e `..` também são pastas, e recursar nelas seria laço.
    fn is_game_folder(&self) -> bool {
        self.is_used() && self.mode & MODE_DIRECTORY != 0 && self.name != "." && self.name != ".."
    }

    fn is_file(&self) -> bool {
        self.is_used() && self.mode & MODE_FILE != 0
    }
}

/// Lê os jogos gravados num cartão de PS2.
///
/// Devolve vazio quando os bytes não são um cartão, quando ele não está formatado, ou quando está
/// vazio. Nunca entra em pânico: isto lê arquivo do disco de outra pessoa.
pub fn read_entries(bytes: &[u8]) -> Vec<CardEntry> {
    let Some(card) = Card::open(bytes) else {
        return vec![];
    };

    let Some(rootdir) = u32_at(&bytes[0..PAGE_DATA], 0x3C) else {
        return vec![];
    };

    // A entrada `.` da raiz declara quantas entradas a raiz tem, e é assim que o próprio PCSX2 a
    // lê (`GetFileEntryPointer`).
    let Some(root) = card.dir_entries(rootdir, 1).into_iter().next() else {
        return vec![];
    };

    card.dir_entries(rootdir, root.length)
        .into_iter()
        .filter(DirEntry::is_game_folder)
        .filter_map(|folder| {
            // Sem código de mídia no nome não dá para dizer de que jogo é a pasta, e inventar
            // seria palpite em cima do save de alguém. É a mesma regra do PS1.
            let serial = super::psx_card::media_code_in(&folder.name)?;
            let title = card
                .dir_entries(folder.cluster, folder.length)
                .into_iter()
                .find(|child| child.is_file() && child.name.eq_ignore_ascii_case(ICON_FILE))
                .map(|icon| card.file_contents(icon.cluster, icon.length))
                .as_deref()
                .and_then(icon_title);

            Some(CardEntry {
                folder: folder.name,
                serial,
                title,
            })
        })
        .collect()
}

/// O nome do jogo, como o próprio jogo o gravou no `icon.sys`.
///
/// O título é uma string só, com uma posição de quebra de linha declarada à parte: o menu do
/// console quebra o nome em duas linhas por causa da largura da caixa, e `GOD OF WAR` chega como
/// `GOD OF` + `WAR`. Aqui as duas metades voltam a ser uma frase.
fn icon_title(bytes: &[u8]) -> Option<String> {
    if !bytes.starts_with(ICON_MAGIC) {
        return None;
    }

    let raw = bytes.get(ICON_TITLE_AT..ICON_TITLE_AT + ICON_TITLE_LEN)?;
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    let raw = &raw[..end];

    let split = u16_at(bytes, ICON_NL_OFFSET_AT)? as usize;
    if split > 0 && split < raw.len() {
        // Best-effort: a unidade do campo não está declarada na fonte, e uma quebra que caia no
        // meio de um caractere Shift-JIS produziria mojibake. Nesse caso vale o título inteiro,
        // que é feio mas verdadeiro.
        if let (Some(first), Some(second)) = (
            text::decode_shift_jis(&raw[..split]),
            text::decode_shift_jis(&raw[split..]),
        ) {
            return Some(format!("{first} {second}"));
        }
    }

    text::decode_shift_jis(raw)
}

#[cfg(test)]
pub(crate) mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// Monta um cartão com os jogos dados, para os testes de integração de `emulator.rs`.
    ///
    /// Exposto porque montar um cartão de PS2 válido tem superbloco, FAT indireta e cadeia de
    /// clusters: duplicar isso num segundo lugar seria duplicar a chance de errar.
    pub(crate) fn card_with(games: &[(&str, &str)]) -> Vec<u8> {
        games
            .iter()
            .fold(CardBuilder::formatted(PAGE_STRIDE_ECC), |card, (folder, title)| {
                card.game(folder, &[("icon.sys", icon_sys(title, None))])
            })
            .build()
    }

    const ROOT_CLUSTER: u32 = 0;
    const ALLOC_OFFSET: u32 = 41;
    const TOTAL_CLUSTERS: u32 = 8135;
    /// Onde a FAT indireta mora, em cluster absoluto. Qualquer lugar fora da área de dados serve.
    const IFC_CLUSTER: u32 = 8;
    const FAT_CLUSTER: u32 = 9;

    /// Monta um cartão válido segundo o formato, para não depender de save de ninguém.
    ///
    /// A FAT é montada de verdade, e não emulada com clusters contíguos: é ela que a leitura
    /// segue, e um teste que a contornasse não provaria nada.
    struct CardBuilder {
        bytes: Vec<u8>,
        stride: usize,
        /// Próximo cluster de dados livre.
        next: u32,
        /// Clusters que formam o diretório-raiz, na ordem da cadeia.
        root: Vec<u32>,
        root_entries: u32,
    }

    impl CardBuilder {
        fn formatted(stride: usize) -> Self {
            let pages = 16_384;
            let mut bytes = vec![0u8; pages * stride];

            bytes[0..MAGIC.len()].copy_from_slice(MAGIC);
            let put16 = |at: usize, value: u16, bytes: &mut Vec<u8>| {
                bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
            };
            put16(0x28, PAGE_DATA as u16, &mut bytes);
            put16(0x2A, 2, &mut bytes);
            for (at, value) in [
                (0x30, TOTAL_CLUSTERS),
                (0x34, ALLOC_OFFSET),
                (0x3C, ROOT_CLUSTER),
                (0x50, IFC_CLUSTER),
            ] {
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }

            let mut card = Self {
                bytes,
                stride,
                next: 1,
                root: vec![ROOT_CLUSTER],
                root_entries: 2,
            };

            // A FAT indireta aponta para o único cluster de FAT que estes testes precisam.
            let indirect = (IFC_CLUSTER * 2) as usize;
            card.page_at(indirect)[0..4].copy_from_slice(&FAT_CLUSTER.to_le_bytes());
            card.chain(&[ROOT_CLUSTER]);

            // A raiz começa com `.` e `..`, como todo diretório do cartão. O `.` declara quantas
            // entradas o diretório tem, e é assim que a leitura descobre o tamanho da raiz.
            let root = card.root.clone();
            card.write_entry(&root, 0, MODE_USED | MODE_DIRECTORY, 2, ROOT_CLUSTER, ".");
            card.write_entry(&root, 1, MODE_USED | MODE_DIRECTORY, 2, ROOT_CLUSTER, "..");
            card
        }

        fn page_at(&mut self, page: usize) -> &mut [u8] {
            let at = page * self.stride;
            &mut self.bytes[at..at + PAGE_DATA]
        }

        /// Escreve a n-ésima entrada de um diretório, resolvendo em que cluster da cadeia ela cai.
        ///
        /// Duas entradas por cluster: errar esta conta é escrever a terceira entrada por cima da
        /// primeira, que é exatamente o defeito que este construtor teve na primeira versão.
        fn write_entry(&mut self, clusters: &[u32], index: u32, mode: u32, length: u32, start: u32, name: &str) {
            let absolute = ALLOC_OFFSET + clusters[(index / 2) as usize];
            let page = (absolute * 2 + index % 2) as usize;
            let entry = self.page_at(page);

            entry.fill(0);
            entry[0x00..0x04].copy_from_slice(&mode.to_le_bytes());
            entry[0x04..0x08].copy_from_slice(&length.to_le_bytes());
            entry[0x10..0x14].copy_from_slice(&start.to_le_bytes());
            entry[0x40..0x40 + name.len()].copy_from_slice(name.as_bytes());
        }

        /// Liga um cluster ao próximo da cadeia, ou marca o fim.
        fn link(&mut self, cluster: u32, next: Option<u32>) {
            let value = match next {
                Some(next) => FAT_IN_USE | next,
                None => FAT_END,
            };
            let page = (FAT_CLUSTER * 2 + (cluster / 128)) as usize;
            let at = (cluster as usize % 128) * 4;
            self.page_at(page)[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }

        /// Grava uma cadeia inteira na FAT.
        fn chain(&mut self, clusters: &[u32]) {
            for (position, cluster) in clusters.iter().enumerate() {
                self.link(*cluster, clusters.get(position + 1).copied());
            }
        }

        fn claim(&mut self) -> u32 {
            let cluster = self.next;
            self.next += 1;
            cluster
        }

        /// Estende uma cadeia até caber `entries` entradas de diretório, deixando um cluster
        /// livre entre um e outro **de propósito**: assim a cadeia não é contígua, e um leitor que
        /// assumisse contiguidade seria reprovado.
        fn grow(&mut self, clusters: &mut Vec<u32>, entries: u32) {
            while (clusters.len() as u32) * 2 < entries {
                let gap = self.claim();
                self.link(gap, None);
                let extra = self.claim();
                clusters.push(extra);
            }
            let chain = clusters.clone();
            self.chain(&chain);
        }

        /// Escreve o conteúdo de um arquivo numa cadeia nova e devolve o cluster inicial.
        fn write_file(&mut self, contents: &[u8]) -> u32 {
            let mut clusters = vec![self.claim()];
            while clusters.len() * PAGE_DATA * 2 < contents.len() {
                clusters.push(self.claim());
            }
            let chain = clusters.clone();
            self.chain(&chain);

            for (position, cluster) in clusters.iter().enumerate() {
                let absolute = ALLOC_OFFSET + cluster;
                for half in 0..2 {
                    let from = position * PAGE_DATA * 2 + half * PAGE_DATA;
                    if from >= contents.len() {
                        break;
                    }
                    let to = (from + PAGE_DATA).min(contents.len());
                    let page = (absolute * 2 + half as u32) as usize;
                    self.page_at(page)[..to - from].copy_from_slice(&contents[from..to]);
                }
            }

            clusters[0]
        }

        /// Acrescenta uma pasta de jogo na raiz, com os arquivos dados.
        fn game(mut self, name: &str, files: &[(&str, Vec<u8>)]) -> Self {
            let entries = 2 + files.len() as u32;
            let mut folder = vec![self.claim()];
            self.grow(&mut folder, entries);

            self.write_entry(&folder, 0, MODE_USED | MODE_DIRECTORY, entries, folder[0], ".");
            self.write_entry(&folder, 1, MODE_USED | MODE_DIRECTORY, entries, ROOT_CLUSTER, "..");

            for (index, (file_name, contents)) in files.iter().enumerate() {
                let start = self.write_file(contents);
                self.write_entry(
                    &folder,
                    2 + index as u32,
                    MODE_USED | MODE_FILE,
                    contents.len() as u32,
                    start,
                    file_name,
                );
            }

            self.root_entries += 1;
            let mut root = self.root.clone();
            self.grow(&mut root, self.root_entries);
            self.root = root.clone();

            self.write_entry(
                &root,
                self.root_entries - 1,
                MODE_USED | MODE_DIRECTORY,
                entries,
                folder[0],
                name,
            );
            // A raiz cresceu: o `.` dela declara o total.
            let total = self.root_entries;
            self.write_entry(&root, 0, MODE_USED | MODE_DIRECTORY, total, ROOT_CLUSTER, ".");

            self
        }

        fn build(self) -> Vec<u8> {
            self.bytes
        }
    }

    /// Monta um `icon.sys` válido segundo a struct `mcIcon`.
    fn icon_sys(title: &str, newline_at: Option<usize>) -> Vec<u8> {
        let mut bytes = vec![0u8; 964];
        bytes[0..4].copy_from_slice(ICON_MAGIC);
        if let Some(at) = newline_at {
            bytes[ICON_NL_OFFSET_AT..ICON_NL_OFFSET_AT + 2].copy_from_slice(&(at as u16).to_le_bytes());
        }
        let encoded = encoding_rs::SHIFT_JIS.encode(title).0.into_owned();
        bytes[ICON_TITLE_AT..ICON_TITLE_AT + encoded.len()].copy_from_slice(&encoded);
        bytes
    }

    #[test]
    fn a_formatted_but_empty_card_has_no_games() {
        assert_eq!(
            Vec::<CardEntry>::new(),
            read_entries(&CardBuilder::formatted(PAGE_STRIDE_ECC).build())
        );
    }

    #[test]
    fn reads_the_serial_and_title_of_a_game() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game(
                "BASLUS-21004MAYC",
                &[("icon.sys", icon_sys("DEF JAM FIGHT FOR NY", None))],
            )
            .build();

        let found = read_entries(&card);

        assert_eq!(1, found.len());
        assert_eq!("BASLUS-21004MAYC", found[0].folder);
        assert_eq!("SLUS-21004", found[0].serial);
        assert_eq!(Some("DEF JAM FIGHT FOR NY".to_string()), found[0].title);
    }

    /// A prova de que a detecção de ECC funciona: o mesmo cartão, nas duas formas de gravação,
    /// tem que dar exatamente o mesmo resultado.
    #[test]
    fn a_card_reads_the_same_with_and_without_ecc() {
        let files = [("icon.sys", icon_sys("GOD OF WAR", None))];

        let with_ecc = read_entries(
            &CardBuilder::formatted(PAGE_STRIDE_ECC)
                .game("BASCUS-97399GodOfWar", &files)
                .build(),
        );
        let without_ecc = read_entries(
            &CardBuilder::formatted(PAGE_STRIDE_PLAIN)
                .game("BASCUS-97399GodOfWar", &files)
                .build(),
        );

        assert_eq!(with_ecc, without_ecc);
        assert_eq!(1, with_ecc.len());
        assert_eq!("SCUS-97399", with_ecc[0].serial);
    }

    #[test]
    fn reads_several_games_from_one_card() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("icon.sys", icon_sys("DEF JAM", None))])
            .game("BESCES-52412JCA", &[("icon.sys", icon_sys("JAK II", None))])
            .build();

        let found = read_entries(&card);

        assert_eq!(2, found.len());
        assert_eq!("SLUS-21004", found[0].serial);
        assert_eq!("SCES-52412", found[1].serial);
    }

    /// A pasta tem mais de duas entradas, então atravessa clusters que não são contíguos. Se a
    /// leitura assumisse contiguidade, o `icon.sys` sumiria e o jogo ficaria sem nome.
    #[test]
    fn finds_a_file_in_a_folder_that_spans_clusters() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game(
                "BASLUS-21004MAYC",
                &[
                    ("save0.bin", vec![1u8; 100]),
                    ("save1.bin", vec![2u8; 100]),
                    ("icon.sys", icon_sys("DEF JAM", None)),
                ],
            )
            .build();

        assert_eq!(Some("DEF JAM".to_string()), read_entries(&card)[0].title);
    }

    /// O `icon.sys` tem 964 bytes e não cabe numa página de 512: se a costura das duas metades
    /// estivesse errada, o título ainda sairia (está no começo) mas o arquivo viria truncado.
    #[test]
    fn reads_an_icon_that_crosses_a_page_boundary() {
        let mut icon = icon_sys("DEF JAM", None);
        icon[960..964].copy_from_slice(b"tail");

        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("icon.sys", icon)])
            .build();

        assert_eq!(Some("DEF JAM".to_string()), read_entries(&card)[0].title);
    }

    #[test]
    fn joins_a_title_broken_into_two_lines() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASCUS-97399GodOfWar", &[("icon.sys", icon_sys("GOD OFWAR", Some(6)))])
            .build();

        assert_eq!(Some("GOD OF WAR".to_string()), read_entries(&card)[0].title);
    }

    #[test]
    fn reads_a_japanese_title() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BISLPM-55108MAYC", &[("icon.sys", icon_sys("ペルソナ", None))])
            .build();

        assert_eq!(Some("ペルソナ".to_string()), read_entries(&card)[0].title);
    }

    /// Sem `icon.sys`, ou com um ilegível, o jogo continua: o serial é a identidade, e o nome é
    /// enfeite. Sumir com o save seria o erro grave.
    #[test]
    fn a_game_without_a_readable_icon_keeps_its_serial() {
        let missing = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("data.bin", vec![7u8; 20])])
            .build();
        let broken = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("icon.sys", vec![0u8; 964])])
            .build();

        for card in [missing, broken] {
            let found = read_entries(&card);
            assert_eq!(1, found.len());
            assert_eq!("SLUS-21004", found[0].serial);
            assert_eq!(None, found[0].title);
        }
    }

    #[test]
    fn ignores_a_folder_without_a_media_code() {
        let card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("SAVEDATA", &[("icon.sys", icon_sys("QUALQUER", None))])
            .build();

        assert_eq!(Vec::<CardEntry>::new(), read_entries(&card));
    }

    #[test]
    fn refuses_a_file_that_is_not_a_ps2_card() {
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&[]));
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&[0u8; 1024]));
        // Tamanho certo, conteúdo lixo.
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0xABu8; 8_650_752]));
        // Um byte a mais e um a menos que o tamanho válido.
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0u8; 8_650_751]));
        assert_eq!(Vec::<CardEntry>::new(), read_entries(&vec![0u8; 8_650_753]));
    }

    /// FAT cíclica não pode travar a varredura.
    #[test]
    fn survives_a_cyclic_fat() {
        let mut card = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("icon.sys", icon_sys("DEF JAM", None))]);
        card.link(ROOT_CLUSTER, Some(ROOT_CLUSTER));
        let bytes = card.build();

        let _ = read_entries(&bytes);
    }

    /// Um cabeçalho mentiroso não pode virar pânico nem leitura fora do arquivo.
    #[test]
    fn survives_a_lying_superblock() {
        let mut bytes = CardBuilder::formatted(PAGE_STRIDE_ECC)
            .game("BASLUS-21004MAYC", &[("icon.sys", icon_sys("DEF JAM", None))])
            .build();

        bytes[0x34..0x38].copy_from_slice(&u32::MAX.to_le_bytes());
        let _ = read_entries(&bytes);

        bytes[0x3C..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
        let _ = read_entries(&bytes);
    }
}
