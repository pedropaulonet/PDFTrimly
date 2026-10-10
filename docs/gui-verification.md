# Verificação do fluxo da interface

Revisão de 10/10/2026. Esta etapa amplia a evidência automática do fluxo GUI;
não encerra a validação nativa exigida para a primeira versão.

## O que foi executado

`src/app_tests.rs` constrói o mesmo `MainWindow` e registra os mesmos callbacks
usados pelo aplicativo. Uma plataforma Slint em memória substitui a janela do
sistema; somente as respostas dos seletores são fornecidas pelo teste. O motor,
as threads, os canais, os timers, os arquivos de entrada/saída e os resultados
são reais. Os PDFs são sintéticos, pertencem ao projeto e ficam em diretórios
temporários. Não há argumentos de teste ou simulações expostos no produto.

| Cenário | Evidência automática |
| --- | --- |
| Seleção múltipla e callback de drop | Caminhos reais, extensão `.PDF`, nome longo com acentos, duplicata e arquivo não PDF |
| Lote completo | PDF compactável e PDF inválido; erro individual não impede terminar o lote |
| Idioma durante o lote | Mudança para francês antes da leitura dos resultados; erros e resumo traduzidos |
| Originais e colisões | Bytes do original e da primeira saída preservados; segundo lote cria nome com sufixo |
| Controles durante o worker | Remover, limpar, adicionar e soltar não alteram as linhas em processamento |
| Cancelamento do seletor | Seleção e destino mantidos; fechamento adiado durante o diálogo |
| Falha do seletor | Desconexão após panic injetado libera o estado de diálogo e mostra erro |
| Erro no destino | Caminho de arquivo usado como pasta causa erro de I/O; lote sai do estado ocupado |
| Fechamento | Cancelar mantém a janela; término durante confirmação a remove; escolha tardia fecha imediatamente; aguardar/parar fecham ao terminar |
| Novo lote | Limpa pedido anterior de fechamento e parada |
| Teclado | Eventos Slint de Ctrl+O, Ctrl+Shift+O e Escape exercem os callbacks reais |
| Parada entre arquivos | Teste do lote solicita parada após `Started`: arquivo atual termina, o próximo não começa e original é preservado |

Comandos executados com sucesso:

```sh
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
cargo check --workspace --locked --offline
```

Total atual: **64 testes**, a etapa original adicionou seis testes de integração da janela e um do
lote. A entrega de preferências acrescentou doze testes de configuração, ações,
renderização e idioma persistido da CLI. O teste de falha do seletor injeta deliberadamente um panic em sua thread;
isso verifica recuperação após desconexão, não indica um crash do produto.

## Preparar a conferência nativa

```sh
cargo run --example gui-fixtures --locked -- /tmp/pdftrimly-gui-check
PDFTRIMLY_LANG=pt-BR cargo run --locked
```

O primeiro comando exige um diretório que ainda não exista e cria `inputs/` e
`outputs/`. Contém um documento compactável com nome longo, um documento pequeno,
um PDF com campo de assinatura, um PDF inválido, um arquivo de texto e uma pasta
com extensão `.pdf`. O campo de assinatura é sintético; não é assinatura real.
Não inclui PDF digitalizado/OCR nem substitui a coleção de documentos reais.
O gerador foi executado nesta revisão e a recusa de pasta existente foi conferida
comparando hashes de todos os arquivos antes/depois.

Na janela, selecione os PDFs de `inputs/`, escolha `outputs/`, compacte e repita.
Confira o resumo, os erros de assinatura/entrada inválida, os sufixos e a
preservação dos originais. Repita usando o gerenciador de arquivos para soltar
PDFs, arquivo de texto e pasta `.pdf`. Complete o roteiro de
[linux-interface.md](linux-interface.md), especialmente teclado, redimensionamento,
parada e fechamento. Para repetir do zero, use outro diretório novo.

## O que permanece pendente

A plataforma em memória não entrega eventos do portal XDG, do gerenciador de
arquivos, do compositor ou do Windows. O callback de drop testado não comprova
entrega do drop pelo sistema. Os eventos de teclado do teste não comprovam
navegação completa por Tab/Shift+Tab, foco visual ou leitores de tela.

Permanecem pendentes operação nativa em X11/Wayland e Windows, conferência nativa de
tema escuro e HiDPI, acessibilidade, comportamento durante PDFs demorados e
avaliação de PDFs reais com OCR e imagens. O motor não foi alterado nesta etapa. Preferências e ações foram adicionadas
na entrega seguinte; veja [settings.md](settings.md) para sua evidência e limites.
Empacotamento continua pendente.

## Pastas e listas grandes

A integração cobre importação recursiva, exclusão ao mudar o destino,
deduplicação entre drop e seletor e saídas nas subpastas correspondentes.
A renderização em memória inclui 2.000 entradas e acesso ao último detalhe.
Testes do lote cobrem colisões relativas e rejeição de escape por caminhos
ou links existentes. Consulte [folder-input.md](folder-input.md).

## Correção da seleção após remoção

Remover uma linha anterior à selecionada agora ajusta seu índice, preservando
o PDF mostrado no painel de detalhes. Remover a própria seleção escolhe a
linha seguinte disponível ou a anterior no fim da lista. Limpar também retorna
a rolagem ao início. O teste de callbacks cobre remoção, limpeza e nova
importação; a suíte completa passou com 65 testes em 10/10/2026. A rolagem
nativa ainda depende de conferência manual.

## Sufixos por idioma e drop nativo

A suíte passou com 66 testes após conectar eventos Winit à entrada e localizar
o sufixo de saída. O teste de integração executa um lote em cada idioma e
confere o nome criado. O cenário existente também troca o idioma durante
o lote e verifica que o nome conserva o idioma inicial. Formato, compilação
e Clippy foram executados. A plataforma em memória não emite eventos do
gerenciador de arquivos: o drop nativo ainda requer conferência em X11/XWayland
e Windows. Wayland puro não oferece esse recurso no Winit atual.

## Organização fixa e revisão dos textos

Aplicadas as escolhas do usuário: subtítulo curto traduzido e manutenção dos
estados Compactado/Sem economia. A lista mostra tamanhos e percentual por PDF,
os detalhes têm rolagem própria e o resumo tem área reservada. Removida a
rolagem externa. Altura mínima: 800 px; tamanho preferido: 960×900. Tamanhos
pequenos usam B/KB para evitar arredondar a zero em MB. O agregado explicita
que os tamanhos são dos PDFs processados com sucesso.

Verificação local: 67 testes passaram; compilação Slint, formato e Clippy sem
erros. As renderizações de configurações usam 680×800 e 960×900, e a lista de
2.000 itens foi renderizada e inspecionada em 680×800. Não foi conferida a
operação nativa deste layout nem os PDFs de outros drives do usuário.

## Comparação de desempenho em PDF real

Em 10/10/2026, um PDF local autorizado pelo usuário, com 26.429.600 bytes e
10 páginas, foi processado em Normal pela CLI. Saída: 9.570.598 bytes,
redução de 63,79%, oito imagens otimizadas e quatro preservadas.
O perfil de desenvolvimento levou 101,795 s; a compilação release atualizada
levou 4,512 s. Esta medição isolada não estabelece um tempo universal.
O usuário relatou cinco minutos na GUI; o processo dessa janela não pôde ser
inspecionado, portanto a causa específica do relato permanece não confirmada.

Para operação normal, preferir `cargo run --release --locked` ou
`./target/release/pdftrimly` após `cargo build --release --locked`.
