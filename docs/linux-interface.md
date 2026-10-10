# Interface Linux

## Compilar e abrir

```sh
source "$HOME/.cargo/env"
cargo build --release --locked
./target/release/pdftrimly
```

Sem argumentos, o executável abre a janela. Os comandos `compress`, `--help`
e `--version` continuam disponíveis. Adicione PDFs pelo seletor ou arraste
arquivos `.pdf` e pastas para a área de soltura da janela. Arquivos repetidos são
ignorados; a extensão é comparada sem diferenciar maiúsculas e minúsculas.
Selecione Leve, Normal ou Ultra, escolha a pasta de saída e clique em
Compactar PDFs. Sem destino selecionado ou salvo, as saídas ficam na pasta
de cada PDF original.

Os PDFs são processados um por vez em um worker. O progresso representa
arquivos concluídos, não o progresso interno do arquivo atual. Cada linha
mostra o nome e estado resumido; o painel de detalhes mantém caminho e
resultado completos com quebra de linha e rolagem. O resumo de tamanhos inclui somente arquivos processados com
sucesso. A parada atua antes do próximo arquivo. Ao fechar durante o lote,
escolha aguardar todo o lote ou parar após o atual; a janela fica aberta até
terminar de publicar uma saída completa. Enquanto um seletor nativo está
aberto, o fechamento da janela é adiado para que a resposta do diálogo possa
ser aplicada ao estado da interface.

Saídas recebem o sufixo do idioma no início do lote (`-compactado.pdf` em
português), com números em caso de colisão.
Após 100 nomes ocupados, informar erro individual. Originais são preservados.

## Backend e dependências

Slint 1.18.1, Winit e renderização por software; X11/Wayland conforme a sessão.
rfd 0.15 usa o portal de arquivos do desktop no Linux, sem GTK incorporado.
O ambiente precisa de sessão gráfica e portal XDG funcional. Fontconfig é
carregado dinamicamente (`libfontconfig.so.1`). O tema usa os widgets e a
preferência do sistema oferecidos pelo backend, com opção persistente Sistema/Claro/Escuro em Configurações.

O drop nativo usa eventos Winit e agrupa múltiplos caminhos; o `DropArea`
também encaminha transferências Slint. No Linux com `DISPLAY`, a janela usa
X11/XWayland, pois Winit não suporta drop de arquivos em Wayland puro.
A entrada aceita caminhos locais com
extensão PDF ou pastas. Consulte [folder-input.md](folder-input.md) para
recursão, exclusão do destino e estrutura relativa. O backend da sessão gráfica precisa encaminhar o drop nativo.
Itens da lista e os seletores ficam desativados durante a compactação porque
alterar o lote em execução poderia associar resultados à linha errada; o botão
Parar continua disponível. A lista e os detalhes têm rolagem própria; cabeçalho, status e ações principais
permanecem visíveis. Nomes longos aparecem completos no painel de detalhes.

O widget AboutSlint na seção Sobre fornece a atribuição. Código do aplicativo
continua MIT; Slint usa os termos de licença próprios para desktop. Não
atribuir MIT às dependências Slint.

## Roteiro manual

1. Selecionar e arrastar múltiplos PDFs, incluindo nomes com espaços e acentos;
   repetir arquivos e conferir deduplicação.
2. Soltar arquivos que não sejam PDFs e conferir que não entram no lote.
3. Remover uma linha; cancelar seletores sem mudar a seleção ou destino.
4. Compactar um lote com digitalizado, documento já otimizado e assinado;
   conferir resultados individuais e continuidade após erro.
5. Repetir na mesma pasta e conferir nomes diferentes, sem sobrescrever.
6. Escolher pasta sem permissão e conferir erro por arquivo.
7. Parar lote e conferir que o arquivo atual termina e os demais não começam.
8. Fechar durante processamento e conferir as opções de fechamento seguro;
   tentar fechar enquanto um seletor está aberto.
9. Redimensionar a janela, navegar pelos controles com Tab/Shift+Tab e conferir
   responsividade durante o processamento de um PDF.
10. Conferir OCR dos PDFs que o possuam e avaliar a qualidade Normal/Ultra.

Essa lista é um roteiro; a operação completa pelos seletores e pelo drop nativo
continua pendente em X11/Wayland e Windows.

## Idiomas e teclado

Inglês, português do Brasil, francês, espanhol e italiano estão incorporados.
O idioma inicial segue o locale do sistema, com fallback inglês. Configurações permite escolher e salvar o idioma entre execuções.
Não há seletor de idioma na tela principal. Consulte [settings.md](settings.md). Para definir o idioma ao iniciar:

```sh
PDFTRIMLY_LANG=pt-BR cargo run --locked
```

Tab/Shift+Tab percorrem os controles; setas e Home/End navegam na lista; Ctrl+O adiciona arquivos e Ctrl+Shift+O
escolhe o destino. Escape fecha Sobre ou cancela a confirmação de fechamento.
A confirmação concentra o foco em manter a janela aberta. Consulte `i18n.md`
para manutenção dos catálogos.

## Evidência desta alteração

Executados `cargo fmt --all --check`, `cargo check --workspace --locked --offline`,
`cargo test --workspace --locked --offline` (64 testes) e
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`.
Os testes cobrem catálogos/CLI, estados e integração dos callbacks da janela com
o worker real, teclado, seletores cancelados/falhos, colisões, parada e
fechamento, além da suíte existente do motor. A plataforma Slint dos testes é
em memória; consulte [gui-verification.md](gui-verification.md) para cenários,
limitações e geração de PDFs sintéticos para a conferência nativa.

Uma janela temporária compilada a partir do Slint real foi renderizada nos cinco
idiomas, com dados fictícios, em 960×760 e 680×620, incluindo nomes longos,
erro e confirmação. Isso revelou e permitiu corrigir alturas das linhas e
acessibilidade dos botões. Essas capturas não validam seletor, drop nativo,
compactação pela GUI, tema escuro, leitores de tela ou Windows. Revisão das
traduções por falantes nativos também permanece pendente.

A tela principal usa status resumido. Copiar resumo mantém o relatório completo.
Clique em uma linha para abrir caminho e resultado; Fechar recolhe os detalhes.
O cabeçalho apresenta somente logo e nome. A altura mínima atual é 740 px.
