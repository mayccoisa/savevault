# Referência visual RepackHub

A fundação v5 descrita abaixo foi aposentada. A escala ativa é a de
[design-system.md](design-system.md), num único módulo (`src/gui/design.rs`). Não há mais
interruptor entre v4 e v5.

O que se aproveitou da comparação: corpo 14, uma cor de destaque escolhida pelo usuário, capa em
proporção 2:3. O que se descartou: título 25, sidebar 264, barra 96, raio 12 e capa de 192 px,
que numa grelha cabiam poucos jogos por linha.

Em 19/10/2026, a pedido do usuário, a fundação v5 passou a seguir o RepackHub instalado localmente.
Referência medida em `resources/app.asar`, arquivo `src/renderer/styles.css`.

| Elemento | RepackHub | SaveVault v5 |
|---|---|---|
| Fonte | Segoe UI Variable Text, Segoe UI | Segoe UI no Windows |
| Corpo | 14 px | 14 px |
| Menu | 13 px, ícones de 18 px | 13 px, ícones de 16 px |
| Título | 25 px | 25 px |
| Sidebar | 264 px | 264 px |
| Margem principal | 32 px | 32 px |
| Cards | capa 2:3, raio 16 px | capa 2:3, raio 16 px |
| Fundos | #08090d, #12151d, #171b25 | mesmos valores |
| Texto | #eef1f8, #9aa3b7 | mesmos valores |
| Acento | #ff3d81 | opção Rosa, preferências existentes preservadas |

O layout da biblioteca e os ícones não dependem mais dessa fundação.
Nenhum mecanismo de backup ou restauração foi alterado por esta rodada visual.

## Validação

401 testes passaram (364 da biblioteca e 37 do binário), com os 24 testes de registro
que exigem preparação externa filtrados. Build de desenvolvimento concluído.
Biblioteca e detalhes conferidos na interface nativa em janela de 1036 px e maximizada.
A referência foi medida no CSS instalado; a comparação visual lado a lado com o
RepackHub não foi realizada porque a autorização para abri-lo expirou.
O cofre de demonstração usa dados sintéticos e destaque rosa.
A restauração de uma versão pela nova tela ainda precisa de validação manual completa.
Clippy em todos os alvos terminou sem erros, com dois avisos anteriores em examples/api.rs e src/scan.rs.
