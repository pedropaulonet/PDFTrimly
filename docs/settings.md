# Preferências e ações do desktop

## Uso

Abra **Configurações**, escolha idioma, tema, compactação padrão e pasta de
saída padrão, inclusão de subpastas e clique em **Salvar**. A gravação aplica os padrões à janela atual
e às próximas execuções. **Cancelar** e Escape descartam o rascunho.

O idioma **Sistema** usa o primeiro locale suportado, com fallback inglês.
O tema **Sistema** usa a preferência fornecida pelo backend Slint; quando ela
é desconhecida, a paleta usa claro. Claro e Escuro podem ser escolhidos
explicitamente. O seletor de idioma na janela principal continua alterando
somente a sessão. O nível e destino dessa janela valem para o lote atual.

`PDFTRIMLY_LANG` prevalece sobre o idioma salvo ao iniciar GUI e CLI. Uma
escolha explícita salva na janela aplica-se imediatamente à sessão, mas a
variável volta a prevalecer no próximo início. A CLI compartilha o idioma
salvo; o nível da CLI continua Normal quando `--level` não é informado.

Depois do lote, **Copiar resumo** copia as estatísticas traduzidas da conclusão.
**Abrir pasta de saída** abre o diretório atual no gerenciador de arquivos.
O Sobre tem ações para o site oficial e apoio: Pix via Livepix em português,
Ko-fi nos demais idiomas. O destino acompanha o idioma atual. Falhas dessas ações
aparecem em um aviso separado; o resumo e resultados permanecem disponíveis.

## Arquivo e recuperação

- Linux: `$XDG_CONFIG_HOME/pdftrimly/preferences.toml`, se XDG_CONFIG_HOME for
  absoluto; caso contrário, `$HOME/.config/pdftrimly/preferences.toml`.
- Windows: `%APPDATA%\pdftrimly\preferences.toml`.
- Sem um diretório válido, o aplicativo funciona com padrões e avisa que não
  pode gravar preferências.

Exemplo da versão 1 do formato:

```toml
version = 1
language = "pt-BR"
theme = "system"
level = "normal"
recursive = false
folder = "/caminho/absoluto/para/saida"
```

`language`: `system`, `en`, `pt-BR`, `fr`, `es`, `it`.
`theme`: `system`, `light`, `dark`. `level`: `light`, `normal`, `ultra`.
`recursive`: booleano; padrão `false`.
`folder = ""` remove a pasta padrão. No Windows, use strings TOML literais
com aspas simples para caminhos com barras invertidas, ou escape as barras.

Campos omitidos recebem padrões. Sintaxe, versão, tipo, valor ou campo
não reconhecido e arquivos maiores que 64 KiB causam recuperação com padrões
e aviso. O arquivo não é alterado ao iniciar. Uma pasta salva que tenha sido
removida não é usada como destino; o aplicativo pede outra pasta.

Salvar exige pasta padrão existente, absoluta e representável em UTF-8, quando
informada. A gravação usa arquivo temporário no mesmo diretório, sincronização
do arquivo e substituição por rename. Erros de validação/gravação não aplicam o
rascunho; a configuração anterior é preservada. Recuperar um arquivo inválido
por meio de Salvar substitui esse arquivo com as escolhas da janela. Não há
migração automática de formatos futuros. Instâncias simultâneas seguem a
última gravação bem-sucedida.

## Implementação e evidência

`src/config.rs` trata o formato e a persistência; `src/settings.rs` trata o
rascunho e callbacks; `src/actions.rs` trata ações; `src/desktop.rs` adapta os
serviços nativos; `src/ui_tasks.rs` executa I/O em workers. O motor permanece
independente. Gravação, seletores, clipboard e abertura de destinos não rodam
na thread gráfica. A janela adia o fechamento durante seletor/gravação das
preferências para não perder a resposta em andamento. Ações externas permitem
fechar a janela e não prendem a compactação.

Os testes verificam roundtrip dos cinco idiomas e três temas/níveis,
recuperação de arquivos inválidos, substituição e preservação em falha,
cancelamento do rascunho, reinício, destino, erros de ações e argumentos enviados
ao serviço do desktop. Há 30 renderizações em memória das configurações:
cinco idiomas × três temas × dois tamanhos (680×620 e 960×760); o teste também
confere que Claro/Escuro alteram os pixels da paleta. Capturas de português
claro e francês escuro foram inspecionadas nesta revisão.

A verificação desta etapa está registrada em [folder-input.md](folder-input.md). Os serviços do desktop são substituídos nos testes: ainda é necessário
conferir clipboard, portal, gerenciador de arquivos e navegador reais em
X11/Wayland e Windows. O clipboard usa arboard e mantém seu provedor enquanto
o aplicativo está aberto; retenção depois de fechar depende do desktop.
A pasta usa `xdg-open` no Linux e `explorer.exe` no Windows; links usam
webbrowser. Textos de diagnóstico do SO podem permanecer no idioma do sistema.

Esta entrega não inclui abertura automática da pasta ao concluir, política configurável de colisões, cancelamento dentro do PDF ou
instaladores. Esses itens continuam no roteiro da primeira versão.
