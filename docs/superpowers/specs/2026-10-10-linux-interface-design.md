# Primeira interface Linux do PDFTrimly

## Objetivo

Permitir compactar vários PDFs numa janela Linux com Slint, usando o motor
Rust existente e mantendo originais e OCR. O usuário confirmou seleção de
múltiplos PDFs. A aplicação continua local e offline.

## Janela e fluxo

Janela redimensionável com logo, título, lista de PDFs e botões Adicionar PDFs
e Remover selecionado. O seletor nativo aceita múltiplos arquivos. Caminhos
duplicados são descartados; cada linha exibe nome, tamanho e estado.

Abaixo da lista: nível Leve / Normal / Ultra, explicação breve da qualidade,
pasta de saída e botão Compactar. Normal é o padrão. Um nível vale para todo
o lote. A interface inicial usa português do Brasil, com textos centralizados
para permitir as traduções previstas na especificação interna posteriormente.

Durante o processamento: arquivo atual, contador de arquivos concluídos e
progresso baseado nesse contador. Não estimar progresso interno por página
porque o motor ainda não oferece esse sinal. Cada linha termina como reduzido,
sem economia, erro ou cancelado antes de iniciar. Mostrar tamanho original,
final e economia de cada arquivo; resumo final informa totais e falhas.

Resultados são gravados como <nome>-compactado.pdf na pasta escolhida. Se o
nome existir, tentar sufixo numérico, preservando a garantia persist_noclobber
do motor. Nunca sobrescrever o original ou saída existente. Assinados e PDFs
criptografados recebem mensagem individual; continuar os demais arquivos.

## Arquitetura

Slint implementa apresentação em ui/main.slint. src/app.rs gerencia estado e
callbacks; um módulo de lote coordena nomes, execução e resultados. A CLI
existente continua disponível. Abrir o executável sem argumentos inicia a GUI.

Seletores de arquivos/pastas usam rfd. Verificar versões, APIs e licenças antes
de adicionar dependências; escolher backend Linux compatível com Wayland/X11.
Incluir a atribuição Slint aplicável na tela Sobre, junto da versão do Cargo,
autor e URLs existentes no projeto. Registrar as condições de distribuição.

Executar um arquivo por vez num worker, separado da thread de interface,
limitando uso de memória. Atualizações chegam à UI pelo mecanismo seguro do
Slint e não acessam widgets diretamente do worker. Capturar falhas por arquivo.
Seleção, nível e destino ficam bloqueados durante o lote.

## Interrupção

O botão Parar lote impede iniciar os próximos arquivos. O arquivo atual
termina normalmente, com indicação explícita disso na interface. Não chamar
essa operação de cancelamento imediato do arquivo: interrupção cooperativa
interna do motor será uma etapa posterior. Ao fechar durante processamento,
oferecer aguardar ou parar após o arquivo atual, preservando saídas completas.

## Escopo da primeira entrega

Seleção múltipla, lista, nível, pasta de destino, execução sequencial em segundo
plano, progresso real por arquivos, resultados e tela Sobre. Usar widgets
acessíveis, foco de teclado e espaçamento consistente. Seguir o tema do sistema
quando suportado pelo backend; documentar o comportamento verificado.

Seleção de pastas de entrada, drag-and-drop, preferências persistentes,
traduções adicionais e instaladores pertencem às próximas entregas. São
requisitos do projeto completo, não funções anunciadas como prontas nesta fase.

## Verificação e aceite

Compilar GUI e preservar CLI. Não acrescentar ou executar testes sem pedido
do usuário. Documentar comandos para ele verificar posteriormente seleção de
vários PDFs, resultados reduzidos/sem economia, erro isolado de um assinado,
colisão de saída, parada entre arquivos e responsividade. Uma execução gráfica
só será declarada verificada quando houver evidência de abertura e interação
numa sessão Linux disponível.

Entregar executável Linux compilado, instrução de abertura e limitações reais.
