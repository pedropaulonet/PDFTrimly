# Proposta de revisão dos textos e da apresentação

Decisões aprovadas: subtítulo traduzido, estados “Compactado / Sem economia”
e organização fixa com lista, detalhes e resumo separados. Aplicados ao Slint,
com tamanho original/final e percentual nas linhas concluídas. A altura mínima
passou a 800 px; tamanho preferido 960×900. Os demais textos da tabela continuam
como propostas para uma próxima rodada de revisão.

## Textos principais

| Área | Inglês proposto | Português proposto |
| --- | --- | --- |
| Subtítulo | Local PDF compression | Compactação local de PDFs |
| Adicionar arquivos | Add PDFs | Adicionar PDFs |
| Adicionar pasta | Add folder | Adicionar pasta |
| Limpar | Clear list | Limpar lista |
| Remover | Remove selected | Remover selecionado |
| Recursão | Include subfolders | Incluir subpastas |
| Entrada por drop | Drop PDFs or folders here | Arraste PDFs ou pastas aqui |
| Durante o drop | Release to add files | Solte para adicionar arquivos |
| Lista vazia | Add PDFs or a folder to begin. | Adicione PDFs ou uma pasta para começar. |
| Seletor de nível | Compression level | Nível de compactação |
| Leve | Light | Leve |
| Ajuda Leve | Keeps images unchanged. Savings may be small. | Mantém as imagens. A redução pode ser pequena. |
| Ajuda Normal | Reduces eligible images. Review visual quality. | Reduz imagens compatíveis. Confira a qualidade visual. |
| Ajuda Ultra | Stronger image reduction, with greater quality loss. | Maior redução de imagens, com maior perda de qualidade. |
| Escolher destino | Choose output folder | Escolher pasta de saída |
| Sem destino escolhido | Save beside each original PDF | Salvar ao lado de cada PDF original |
| Iniciar | Compress PDFs | Compactar PDFs |
| Parar | Stop after current PDF | Parar após o PDF atual |
| Resultado com redução | Size reduced | Tamanho reduzido |
| Resultado sem redução | No reduction | Sem redução |
| Falha individual | Failed | Falhou |
| Detalhes vazios | Select a PDF to view its details. | Selecione um PDF para ver os detalhes. |
| Copiar | Copy summary | Copiar resumo |
| Abrir destino explícito | Open output folder | Abrir pasta de saída |
| Privacidade | Files stay on your computer. Originals are preserved. | Os arquivos ficam no seu computador. Os originais são preservados. |

Os detalhes técnicos de DPI/JPEG podem permanecer em ajuda ou configurações,
sem ocupar permanentemente a área principal. A preservação do OCR continua
documentada; sai apenas do subtítulo. As mensagens de falha devem identificar
arquivo e motivo, preservando os diagnósticos úteis.

## Lista e resumo

Proposta: controles de entrada e destino fixos; lista com rolagem própria;
detalhes do PDF selecionado separados; resumo com altura suficiente para
mostrar conclusão, contagens, tamanhos e percentual, sem cortar linhas.

Cada linha deve mostrar nome, estado, tamanho original, tamanho final e
percentual. Caminhos completos e mensagens extensas ficam nos detalhes.
A seleção precisa permanecer visível ao navegar ou remover itens.

Exemplo de resumo:

```text
Lote concluído: 51 de 51 PDFs
31 com redução · 13 sem redução · 7 com falha
Arquivos processados com sucesso: 341,54 MB → 341,28 MB
Redução total: 0,08%
Itens ignorados na importação: 199
```

Os números deste exemplo foram lidos da captura; não constituem uma
verificação dos PDFs do usuário. “Ignorados” inclui duplicatas, arquivos não
PDF e diretórios/links não percorridos; deve haver explicação acessível dessa
contagem para não parecer que 199 PDFs falharam.

## Conferência do cálculo

O motor mede a entrada pelo comprimento dos bytes efetivamente lidos e a
saída pelo tamanho do arquivo temporário gravado. A redução é
`(entrada - saída) / entrada × 100`. O agregado inclui apenas resultados de
sucesso, incluindo os sem redução. Arquivos com falha não entram nos tamanhos.
Não há cálculo específico por drive. A apresentação em MB arredonda para duas
casas; detalhes com bytes e tamanho economizado ajudariam a conferir reduções
pequenas.

Para conferir o lote da captura: obter o texto copiado do resumo e os tamanhos
em bytes de pelo menos um par original/saída, além do nível usado. A inspeção
da captura e do código não confirma a integridade desses arquivos reais.

## Decisões pendentes

- Traduzir o subtítulo conforme o idioma ou mantê-lo sempre em inglês.
- Usar “Tamanho reduzido / Sem redução” ou “Compactado / Sem economia”.
- Aprovar a organização fixa com lista, detalhes e resumo separados.

Após as escolhas, aplicar o mesmo padrão aos cinco catálogos e revisar
mensagens de seleção, progresso, fechamento, preferências, ações e erros.

## Níveis de compactação

O usuário aprovou manter Leve / Normal / Ultra. As explicações foram
simplificadas nos cinco idiomas: Leve mantém as imagens e pode ter redução
pequena; Normal reduz imagens compatíveis e pode diminuir a qualidade visual;
Ultra aplica maior redução a essas imagens, com maior perda de qualidade.
Os parâmetros DPI/JPEG continuam documentados em `image-optimization.md`.

## Contagem da importação

Aprovado e aplicado: “Itens não adicionados”, com explicação de duplicatas,
arquivos não PDF, pastas não percorridas e itens indisponíveis. A contagem
também inclui links não percorridos. Essa mensagem foi traduzida nos cinco
idiomas; a contagem e as regras de descoberta não mudaram.

## Remoção da seleção

Aprovado e aplicado nos cinco idiomas: “Remover da lista”. A ação continua
removendo somente a entrada selecionada; não apaga o PDF do computador.

## Simplificação aprovada da tela principal

Idioma fica exclusivamente em Configurações. Cabeçalho mantém logo e nome,
sem subtítulo. Entrada reduzida a 72 px. Lista apresenta nome, tamanho original
e estado/percentual; o caminho e o resultado completos ficam em detalhes
abertos ao clicar na linha, com botão Fechar. Nenhum espaço para detalhes é
reservado enquanto estiverem fechados.

O status apresenta quantidade processada, percentual agregado e erros; Copiar
resumo conserva o relatório completo e a contagem de itens não adicionados.
Compactar PDFs é a única ação principal e tem maior espaço e altura.
A janela atual tem altura mínima de 740 px e tamanho preferido 940×820.
Os níveis e seus efeitos permanecem visíveis antes de iniciar o lote.
