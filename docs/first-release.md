# O que falta para a primeira versão

Preparação da **0.1.1**, revisada em 11/10/2026. Cargo e lockfile estão em
0.1.1; isso não indica publicação ou aprovação dos critérios de distribuição.

## Estado implementado

| Área | Código disponível | Evidência e limite |
| --- | --- | --- |
| Motor local | Leve, Normal e Ultra; imagens elegíveis; OCR existente; limites; saída temporária; ausência de sobrescrita | Testes sintéticos de integridade, redução, imagens, assinaturas e criptografia. Equivalência visual e cobertura de documentos reais ainda precisam ser ampliadas. |
| Lote pela GUI | Seleção múltipla, arquivos/pastas por drop, descoberta recursiva, deduplicação, destino, nível, worker sequencial, resultados individuais | Estados, canais, callbacks da janela, timers e worker real cobertos; operação nativa pelos seletores e gerenciador de arquivos ainda requer roteiro manual. |
| Interface | Logo, paleta do estilo, lista com rolagem, nomes longos, estados vazios, controles reais, Sobre e confirmação de fechamento | Compilação Slint e capturas com dados fictícios nos cinco idiomas em 960×760 e 680×620; configurações também renderizadas em claro/escuro; interação nativa, Windows, HiDPI e leitores de tela ainda pendentes. |
| Idiomas | Cinco catálogos incorporados; detecção de locale; fallback inglês; troca na janela; textos da CLI | Chaves, parâmetros, locale, plurais, interpolação e CLI cobertos. Idioma padrão persistido; revisão com falantes nativos ainda pendente. |
| Resumo | Quantidades com/sem economia, erros, tamanhos comparáveis e percentual agregado | Dados de falhas não entram no total de tamanhos. Cópia do resumo e abertura da pasta implementadas; clipboard/launcher nativos ainda requerem conferência. |
| Preferências | Idioma, Sistema/Claro/Escuro, nível, subpastas e pasta padrão; Salvar/Cancelar; TOML versionado com substituição atômica | Persistência, reinício, recuperação e erros cobertos. Ações nativas e caminhos Windows ainda precisam de conferência. |
| CI | Workflow com Linux e Windows: formato, compilação, Clippy e testes | Primeira execução bem-sucedida no commit `029a594`, conforme resultado do GitHub Actions fornecido pelo autor. |

## Pendências prioritárias

### 1. Validar a operação completa e a segurança dos resultados

Executar o roteiro de `docs/linux-interface.md` em X11 e Wayland e em Windows:
seletores, drop nativo, nomes longos, teclado, mudança de idioma durante o lote,
saídas existentes, pasta sem permissão, parada e fechamento exatamente quando
o último arquivo termina. Ampliar a coleção autorizada de PDFs reais e
comparar abertura, texto/OCR e renderização das saídas. Não versionar documentos
privados. Critério: evidência reproduzível e nenhuma perda de original ou saída
incompleta, dentro dos recursos suportados.

### 2. Concluir os controles e preferências previstos

Configurações com idioma, tema, nível e pasta padrão foram implementadas com
persistência TOML e recuperação de arquivo inválido. Copiar resumo, abrir pasta
e os links do Sobre também estão conectados. Consulte [settings.md](settings.md).
Restam a conferência nativa desses controles e as opções previstas para comportamento configurável de colisões e abertura automática ao concluir. Essas
opções devem acompanhar a implementação correspondente, com ações reais.

### 3. Entrada por pasta e relatórios completos

Descoberta em worker, exclusão do destino, deduplicação canônica, estrutura
relativa e contagem de ignorados estão implementadas. A lista compacta dispõe
de painel de detalhes e navegação por teclado. Consulte
[folder-input.md](folder-input.md). Falta conferir o fluxo nativo com pastas
reais, permissões e lotes grandes em Linux/Windows.

### 4. Cancelamento cooperativo no motor

Parar atualmente impede iniciar o próximo arquivo; o PDF atual termina. O
motor não recebe token de cancelamento e não tem checkpoints durante leitura,
recompressão e validação. Critério: cancelamento em pontos seguros, remoção de
temporários e confirmação de fechamento sem aguardar um PDF inteiro quando a
operação puder ser interrompida com segurança.

### 5. Preparar e verificar a distribuição

O workflow `packages.yml` gera EXE para Windows e DEB/AppImage para Linux x86_64,
com desktop entry, ícone SVG, metadados, textos de licença e checksums. Ele roda
em push para `main`, tags `v*` ou acionamento manual, disponibilizando artefatos
por 30 dias. Confira [packaging.md](packaging.md).
Tags `v<versão>` publicam após os dois builds, conforme autorização do autor. Windows deve ser
distribuído com o ZIP completo; os checksums incluem textos e avisos.
A coleta usa a árvore ativa por plataforma e gera `NOTICE-AUDIT.json`;
bibliotecas do AppImage sem origem identificada interrompem o empacotamento.
Veja a auditoria e as limitações em [dependencies.md](dependencies.md).

O [roteiro em ambientes limpos](clean-install.md) define a matriz e o registro
reproduzível de instalação, inicialização e operação. **Ainda não executado**
em VMs limpas Linux/Windows. A revisão completa do runtime AppImage e das
bibliotecas nativas continua necessária. Critério: avisos completos e evidência
por artefato; nenhum teste do runner substitui instalação na VM.

## Verificações da preparação 0.1.1

Consulte [release-0.1.1-verification.md](release-0.1.1-verification.md) para os
comandos executados, os 69 testes Rust, a coleta de avisos por plataforma e o
ensaio local de construção/extração do DEB. Não houve instalação em VM limpa,
execução Windows ou construção do AppImage nesta revisão.

## Evidência adicional do fluxo GUI

A revisão automatizada passou com 64 testes, incluindo integração da janela com
o motor e fechamento em diferentes ordens de eventos. O relatório
[gui-verification.md](gui-verification.md) registra cenários e limitações e fornece
um gerador de PDFs sintéticos para a verificação nativa. Ainda não há evidência
de conclusão do roteiro manual em Linux/Windows.

## Próxima entrega recomendada

Primeiro concluir a verificação gráfica ponta a ponta e corrigir o que ela
revelar, incluindo clipboard, tema, links e preferências em ambiente nativo.
A próxima implementação recomendada é cancelamento cooperativo dentro do
PDF. Empacotamento pode seguir como entrega própria. A publicação da 0.1.1 deve aguardar os
critérios da especificação ou uma decisão explícita de reduzir o escopo do
primeiro lançamento e documentar o novo escopo.
