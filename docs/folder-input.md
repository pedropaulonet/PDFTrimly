# Entrada por pastas e listas grandes

Use **Adicionar pasta** ou solte PDFs e pastas na área de entrada. Marque
**Incluir subpastas** antes de importar para descobrir PDFs recursivamente.
A opção começa desmarcada e pode ser salva em Configurações. Alterá-la não
refaz uma seleção existente; adicione novamente a pasta se necessário.

A descoberta roda em worker, com ordem determinística, extensão PDF sem
sensibilidade a maiúsculas e deduplicação por caminho canônico. Links de
pastas não são percorridos; links encontrados dentro de pastas são ignorados.
Os controles de seleção aguardam o fim da descoberta e o aviso de trabalho
permanece visível. O fechamento aguarda a resposta desse worker. Não há
cancelamento interno da descoberta nesta versão.

## Destino e estrutura

Sem destino escolhido ou salvo nas preferências, cada saída fica ao lado do
PDF original, inclusive quando os PDFs vieram de subpastas. Por exemplo,
`/entrada/contratos/a.pdf` produz `/entrada/contratos/a-compactado.pdf` em
português. O botão Compactar fica disponível sem escolher destino. Uma pasta
padrão salva nas Configurações continua sendo um destino explícito.

Nesse modo, não há uma pasta exclusiva de saída para excluir da descoberta:
reimportar uma pasta pode incluir PDFs compactados anteriormente. A seleção
atual não é ampliada automaticamente durante o processamento.


Para uma pasta `/entrada`, `/entrada/contratos/a.pdf` produz
`/saida/contratos/a-compactado.pdf` (em português). O nome da pasta raiz não é acrescentado.
Várias raízes compartilham o destino; colisões recebem sufixos numéricos,
sem sobrescrever saídas ou originais. Após 100 nomes ocupados há erro individual.

A pasta de saída e seus descendentes são excluídos da descoberta. Ao escolher
ou aplicar outro destino, entradas descobertas em pastas que estejam dentro
dele são removidas; a exclusão é repetida antes de iniciar o lote. PDFs
adicionados explicitamente podem usar o mesmo diretório como destino.

Em importações sobrepostas, a primeira origem de cada PDF é mantida: um
arquivo adicionado diretamente continua com saída na raiz; um arquivo
adicionado por pasta mantém a estrutura relativa àquela primeira pasta.
Para mudar a origem, remova o arquivo e importe pela pasta desejada.

O resumo conta itens ignorados, incluindo duplicatas, arquivos não PDF,
links, diretórios não percorridos e falhas de descoberta. Uma pasta excluída
conta como um item; seus descendentes não são enumerados. Falhas de descoberta
aparecem no aviso agregado de importação; não têm resultados individuais.
Caminhos relativos de saída são restringidos à raiz de destino, incluindo
verificação de links existentes. Alterações concorrentes no sistema de arquivos
não são protegidas por uma transação de diretórios.

## Lista e teclado

A lista usa linhas compactas com nome e tamanho/estado resumido, rolagem e
instanciação das linhas visíveis. Clique numa linha para ler caminho completo,
mensagem de erro ou resultado no painel de detalhes com rolagem. Tab entra
na lista; setas para cima/baixo e Home/End mudam a seleção e acompanham a linha.
Remover atua na seleção. Durante o lote é possível consultar os detalhes;
edição da seleção aguarda o fim do processamento.

A janela tem altura mínima de 800 px. Os controles e o resumo ficam fixos;
a lista e os detalhes têm suas próprias rolagens. Nomes truncados na linha permanecem completos nos detalhes.

## Evidência e limites

Testes automatizados cobrem recursão, exclusão de destino, duplicatas, links
no Linux, estrutura relativa, colisões e rejeição de caminhos de saída que
escapam da raiz. A integração da janela usa o worker e motor reais em uma
plataforma Slint em memória. Uma lista com 2.000 nomes longos é renderizada
nesse ambiente e o detalhe do último item é conferido. Uma captura foi
inspecionada para a apresentação compacta.

Seletores, drop nativo, teclado e desempenho em um desktop real ainda precisam
ser conferidos em X11/Wayland e Windows. Execute `cargo run --locked` e siga
[linux-interface.md](linux-interface.md). O motor continua local e sequencial;
Parar aguarda o PDF atual terminar.

Verificação local em 10/10/2026: `cargo test --workspace --locked --offline`
passou com 64 testes; `cargo fmt --all --check`,
`cargo check --workspace --locked --offline` e
`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
terminaram com sucesso. A captura adicional da lista de 2.000 itens foi
renderizada e inspecionada em 680×620. Esses resultados não substituem a
verificação nativa descrita acima.

## Drop nativo e sufixos por idioma

O aplicativo conecta `WindowEvent::DroppedFile` do Winit ao mesmo fluxo de
descoberta do seletor. Eventos consecutivos são agrupados por um timer de
50 ms para aceitar múltiplos caminhos. O drop vale em qualquer ponto da janela
quando a seleção pode ser editada. O `DropArea` continua disponível para
backends que encaminhem transferências Slint.

Winit 0.30 não oferece drop de arquivos em Wayland. No Linux, quando `DISPLAY`
está disponível, o aplicativo seleciona X11 (XWayland em sessão Wayland).
Uma sessão exclusivamente Wayland sem XWayland ainda exige usar os seletores.
O comportamento real do gerenciador de arquivos deve ser conferido no desktop.

O sufixo é definido pelo idioma no início de cada lote: inglês `compressed`,
português `compactado`, francês `compresse`, espanhol `comprimido` e italiano
`compresso`. Trocar idioma durante o lote traduz a interface; os nomes daquele
lote conservam o idioma inicial. Arquivos já gravados não são renomeados.
