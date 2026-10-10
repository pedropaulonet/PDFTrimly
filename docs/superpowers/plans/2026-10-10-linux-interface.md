# Interface Linux — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Compactar múltiplos PDFs numa interface Linux responsiva.

**Architecture:** Slint apresenta a fila e recebe eventos do controlador Rust. Um worker executa o motor sequencialmente e comunica resultados à thread gráfica. A CLI e o crate independente permanecem disponíveis.

**Tech Stack:** Rust, Slint, rfd, pdftrimly-core existente; versões e backends confirmados antes de integração.

**Spec:** docs/superpowers/specs/2026-10-10-linux-interface-design.md

## Global Constraints

- Processamento local e offline; originais e saídas existentes preservados.
- Seleção múltipla; um nível por lote; Normal como padrão.
- Saída `<nome>-compactado.pdf`, com sufixo numérico em colisões.
- Um arquivo por vez fora da thread de UI.
- Progresso por arquivos concluídos, sem estimativa fictícia interna.
- Parar lote impede próximos arquivos; o atual termina normalmente.
- Primeira interface em português do Brasil; textos centralizados.
- Não adicionar nem executar testes sem pedido do usuário; compilar e documentar verificações manuais.

## Review Focus

- Caminhos com espaços, Unicode e nomes iguais em pastas distintas: preservar identificação e evitar colisões.
- Diretório sem permissão e saída criada por outro processo: informar erro individual sem sobrescrever.
- PDF assinado, criptografado ou inválido: continuar arquivos seguintes.
- Fechar ou parar durante processamento: preservar saída completa e não iniciar próximo arquivo.
- Seleção cancelada e fila vazia: manter estado coerente e impedir início inválido.

## Task 1 — Coordenação do lote

**Files:** criar src/batch.rs e src/messages.rs.

**Interfaces:** `BatchRequest { inputs: Vec<PathBuf>, output_dir: PathBuf, level: CompressionLevel }`; `BatchEvent::{Started { index: usize }, Finished { index: usize, result: Result<OptimizeReport, String> }, Completed { stopped: bool }}`. `run_batch(request: BatchRequest, stop: Arc<AtomicBool>, emit: impl FnMut(BatchEvent))`.

- [x] Centralizar textos, rótulos e mensagens visíveis em src/messages.rs.
- [x] Implementar geração de nomes com sufixo numérico e criação exclusiva do motor; tratar colisão concorrente com nova tentativa limitada a 100 alternativas.
- [x] Implementar worker sequencial com eventos por arquivo, tratamento de panic isolado e verificação da flag antes de cada arquivo.
- [x] Documentar casos manuais de Unicode, colisões, permissão e documento rejeitado; não executar testes.

## Task 2 — Janela e integração

**Files:** criar ui/main.slint, src/app.rs e build.rs; modificar Cargo.toml, Cargo.lock e src/main.rs.

**Interfaces:** `app::run() -> Result<(), Box<dyn std::error::Error>>`; `MainWindow` com modelo de linhas contendo nome, caminho, estado, tamanho original/final e economia; callbacks adicionar, remover, escolher destino, iniciar, parar e Sobre.

- [x] Confirmar APIs, versões, backend Linux e licença/atribuição de Slint e rfd em fontes oficiais; adicionar dependências e compilação Slint.
- [x] Criar janela redimensionável com logo, lista de PDFs, seletor de nível, destino, resumo, progresso por arquivos e Sobre.
- [x] Conectar seleção múltipla, deduplicação por caminho canônico e remoção da linha selecionada; cancelamento de diálogos preserva estado.
- [x] Implementar controlador, diálogos fora do bloqueio da UI e entrega dos eventos pelo event loop do Slint; bloquear alterações durante o lote e habilitar parada.
- [x] Tratar fechamento ativo com confirmação para aguardar ou parar após o arquivo atual; fechar apenas depois de conclusão segura.
- [x] Abrir GUI sem argumentos e preservar comandos CLI; adaptar texto de ajuda.
- [x] Compilar workspace e release, corrigindo diagnósticos de compilação sem executar testes.

## Task 3 — Entrega Linux

**Files:** atualizar README.md, docs/architecture.md, docs/dependencies.md e criar docs/linux-interface.md.

- [x] Registrar dependências, atribuição e condições de licença; documentar tema e limitações verificadas do backend.
- [x] Documentar abertura do executável e roteiro manual: seleção múltipla, ausência de economia, falha isolada, colisões, parada, fechamento e responsividade.
- [x] Abrir GUI somente se a sessão Linux e permissões disponíveis permitirem; separar evidência de compilação de evidência de interação.
- [x] Entregar caminho do executável release e informar precisamente o que foi compilado e o que ainda requer verificação manual.
