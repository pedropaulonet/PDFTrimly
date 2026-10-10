# PDFTrimly — Especificação de desenvolvimento

**Versão do documento:** 1.1  
**Versão inicial do aplicativo:** 0.1.0  
**Autor:** Pedro Paulo Reis  
**Site:** https://dev.pedropaulo.net/pdftrimly  
**Apoio:** https://ko-fi.com/pedropaulo


## esse será o repositório

git remote add origin git@github.com:pedropaulonet/PDFTrimly.git
git branch -M main
git push -u origin main


## 1. Objetivo e princípios

Desenvolver **PDFTrimly**, aplicativo desktop para compactar e otimizar PDFs em Windows e Linux, com interface moderna, limpa e responsiva. O projeto deve seguir **KISS**, priorizando confiabilidade, desempenho, acessibilidade, manutenção simples e processamento **100% local e offline**.

- Implementar o aplicativo e seu motor em Rust; usar Slint para a interface, após verificar a licença adequada à distribuição pretendida.
- Não utilizar Ghostscript, Poppler, MuPDF, executáveis externos ou serviços de terceiros **para realizar a compactação**. Ferramentas de empacotamento e validação nos testes/CI podem ser usadas separadamente, sem integrarem o motor.
- Preferir bibliotecas Rust e evitar dependências nativas complexas.
- Nunca modificar o arquivo PDF original.
- Não incluir anúncios, telemetria ou upload de documentos.
- Respeitar tema claro/escuro do sistema e funcionar corretamente em janelas redimensionáveis no Windows e no Linux.
- Usar a captura de tela fornecida como referência de organização visual, aperfeiçoando espaçamentos, tipografia, alinhamento e acabamento.

## 2. Identidade

- **Nome público:** PDFTrimly
- **Nome técnico / repositório / executável:** `pdftrimly`
- **Autor:** Pedro Paulo Reis
- **Site:** https://dev.pedropaulo.net/pdftrimly
- **Apoio:** Ko-fi — https://ko-fi.com/pedropaulo
- **Versão inicial:** `0.1.0`, obtida de `Cargo.toml`
- **Licença desejada:** MIT, **condicionada** à compatibilidade das dependências e às obrigações aplicáveis ao Slint. Verificar antes de fixar a licença e distribuir binários.

Não inventar URLs, credenciais, dados do autor ou registros de marca. A disponibilidade jurídica do nome deve ser verificada separadamente.

## 3. Stack e arquitetura

Usar Rust stable, Cargo e, após validação de licença, Slint. Investigar e selecionar versões mantidas de `lopdf` (estrutura PDF), `flate2` (fluxos), `image` e codecs JPEG adequados (imagens), `serde`/`toml` (configurações), `sys-locale` (idioma), `directories` (diretórios do usuário), `rfd` (seleção de arquivos e pastas) e uma solução de logging.

**Não assumir** que `lopdf` ou `image` resolvem automaticamente imagens PDF com CMYK, máscaras, transparência, filtros especiais ou assinaturas. Confirmar APIs, licenças, formatos suportados e limitações antes da implementação.

Separar o motor da GUI em um crate Rust independente, testável e futuramente reutilizável por uma CLI.

```text
pdftrimly/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── assets/
│   ├── icon.png
│   └── icon.ico
├── crates/
│   └── pdftrimly-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── optimizer.rs
│           ├── images.rs
│           ├── streams.rs
│           └── validation.rs
├── packaging/
│   └── linux/
│       └── pdftrimly.desktop
├── scripts/
│   └── build-local.sh
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── config.rs
│   └── i18n.rs
├── ui/
│   ├── main.slint
│   ├── settings.slint
│   └── about.slint
├── i18n/
│   ├── en.toml
│   ├── pt-BR.toml
│   ├── fr.toml
│   ├── es.toml
│   └── it.toml
├── tests/
│   └── fixtures/
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── .gitignore
├── LICENSE
└── README.md
```

A estrutura é uma proposta inicial: adaptar somente quando houver justificativa técnica documentada.

## 4. Prova de conceito obrigatória do motor

**Antes de implementar a interface completa**, criar uma prova de conceito executável em Rust que:

1. Receba um PDF de entrada e um caminho de saída.
2. Analise a estrutura e identifique oportunidades concretas de otimização.
3. Regrave e otimize com técnicas efetivamente implementadas, sem chamar compactadores externos.
4. Preserve o conteúdo e os recursos suportados, especialmente texto pesquisável.
5. Valide estruturalmente a saída e compare número de páginas.
6. Meça tamanho original, tamanho final, economia percentual e duração.
7. Documente quais otimizações foram realizadas e quais não são suportadas.
8. Não apresente regravação sem redução como sucesso de compactação.

Usar PDFs de texto, digitalizações, fotografias, gráficos vetoriais e conteúdo misto. Não impor redução mínima universal: PDFs já otimizados podem não diminuir.

**Critério de aceite:** o motor processa uma coleção representativa sem corromper documentos suportados, relata corretamente ausência de economia e demonstra redução real em pelo menos alguns arquivos apropriados.

## 5. Níveis de compactação

### Leve — sem perda visual intencional
- Remover somente objetos comprovadamente não utilizados.
- Reorganizar objetos e estruturas internas quando seguro.
- Recompactar fluxos elegíveis apenas quando houver benefício.
- Não recomprimir imagens com perda.
- Preservar textos, vetores, metadados relevantes, links, anotações e recursos suportados.

### Normal — equilíbrio
- Aplicar otimizações estruturais seguras do modo Leve.
- Para imagens elegíveis, reduzir a resolução **efetiva** quando ultrapassar aproximadamente 150 DPI.
- Utilizar JPEG com qualidade inicial aproximada de 80% quando apropriado e vantajoso.
- Não converter indiscriminadamente imagens com máscaras, transparência, CMYK ou outros recursos complexos.

### Ultra — tamanho menor
- Aplicar otimizações estruturais seguras.
- Para imagens elegíveis, usar resolução efetiva de referência de aproximadamente 96 DPI.
- Usar qualidade JPEG inicial aproximada de 60% quando apropriado.
- Aceitar perda visual controlada, sem destruir legibilidade ou elementos essenciais.

Os valores de DPI e qualidade são **pontos de partida ajustáveis após testes**, não garantias de redução nem autorização para degradar todos os tipos de imagem.

## 6. Integridade e segurança documental

- Nunca sobrescrever PDFs originais.
- Não rasterizar páginas inteiras indiscriminadamente; preservar texto pesquisável sempre que possível.
- Detectar criptografia e rejeitar com mensagem clara os casos não suportados.
- Detectar assinaturas digitais e **não alterar PDFs assinados por padrão**; não prometer preservar a validade de assinatura após regravação.
- Preservar formulários, anexos, marcadores, anotações, links e metadados relevantes, quando suportados. Rejeitar ou pular arquivos cujos recursos não possam ser preservados com segurança.
- Usar arquivo temporário na pasta de destino, validar e só então promover ao nome final por operação segura. Limpar temporários após falhas ou cancelamento.
- Nunca sobrescrever saída existente sem política expressamente escolhida.
- Se o resultado for maior ou igual ao original, copiar o original para a saída (quando permitido) e informar **sem economia**.
- Isolar falhas por arquivo, tratar PDFs malformados e evitar travamento da aplicação.
- Validar estrutura, contagem de páginas e objetos essenciais; reconhecer que validação estrutural **não garante equivalência visual perfeita**.
- Impor limites razoáveis de memória, tempo e tamanho de imagens descomprimidas, para reduzir riscos com arquivos hostis.
- Implementar cancelamento cooperativo e logs diagnósticos sem registrar conteúdo confidencial dos documentos.

## 7. Interface gráfica

Usar a imagem de referência enviada para a disposição geral, mas modernizar o acabamento e manter a interface KISS.

**Janela principal:**
- Nome PDFTrimly e ícone discreto no cabeçalho.
- Botões Configurações e Sobre.
- Seleção de arquivos PDF individuais e/ou pasta de entrada, com diálogos nativos.
- Arrastar e soltar arquivos e pastas, se tecnicamente viável nas plataformas-alvo; testar o comportamento real.
- Seleção da pasta de saída.
- Escolha exclusiva: **Leve / Normal / Ultra**, com explicações curtas.
- Botões **Começar** e **Parar**, com estados corretos.
- Barra de progresso real, nome do arquivo atual, quantidade processada, tamanho original/final e economia.
- Resumo final e botão para abrir a pasta de saída.

**Visual:**
- Interface limpa, espaçamento consistente, tipografia legível e bordas discretamente arredondadas.
- Tema **Sistema / Claro / Escuro**, detectando a preferência do SO quando suportado; fallback definido quando indisponível.
- Janela redimensionável, tamanho mínimo adequado, boa escala HiDPI e navegação por teclado.
- Textos e controles preparados para traduções maiores.
- Sem gradientes excessivos, animações desnecessárias ou botões decorativos sem função.
- Compactação executada fora da thread de UI, com atualizações seguras e cancelamento real.

## 8. Processamento em lote

- Aceitar seleção de arquivos, pasta e, opcionalmente, subpastas.
- Processar **sequencialmente** no MVP.
- Preservar nomes e estrutura relativa de subpastas quando aplicável.
- Ignorar a pasta de saída durante a descoberta, inclusive quando estiver dentro da entrada.
- Prevenir caminhos duplicados e colisões de nomes.
- Continuar após falha isolada e apresentar o motivo por arquivo.
- Permitir cancelamento seguro, sem arquivos finais incompletos.
- Evitar iniciar duas operações simultâneas sobre o mesmo conjunto de arquivos.
- Exibir resumo: encontrados, compactados com economia, sem economia, ignorados, erros, bytes originais, bytes finais e percentual de economia.
- Calcular estatísticas apenas sobre conjuntos comparáveis e contabilizar corretamente os arquivos copiados sem redução.
- Permitir copiar o resumo para a área de transferência.

## 9. Configurações

Salvar preferências em TOML no diretório de configuração recomendado pelo SO.

**Geral:** idioma, tema, nível de compactação padrão, pasta de saída padrão, incluir subpastas, comportamento quando saída já existe e abrir pasta de saída ao concluir.

**Compactação:** explicar cada nível; manter parâmetros avançados fora da interface inicial, mas permitir expansão futura.

Na primeira execução, detectar idioma e tema do sistema. Persistir alterações feitas pelo usuário e tratar configurações inválidas sem falhar ao iniciar.

## 10. Internacionalização

Incluir traduções completas para:

- Inglês (`en`)
- Português do Brasil (`pt-BR`)
- Francês (`fr`)
- Espanhol (`es`)
- Italiano (`it`)

O idioma inicial segue o idioma do SO; usar inglês como fallback quando não houver correspondência. Permitir mudança nas configurações. Traduzir botões, menus, avisos, erros, progresso, About, configurações e relatórios. Não espalhar strings traduzíveis diretamente pela UI. Validar que nenhuma chave de tradução está ausente.

## 11. Janela Sobre

Exibir:

- **PDFTrimly**
- Versão obtida de `Cargo.toml`.
- **Desenvolvido por Pedro Paulo Reis**.
- Site: https://dev.pedropaulo.net/pdftrimly
- Botão **Apoiar no Ko-fi**: https://ko-fi.com/pedropaulo
- Licença efetivamente adotada.
- Mensagem de privacidade: processamento local, sem envio de PDFs a servidores.
- Quando útil, arquitetura e sistema operacional da compilação.

Não implementar telemetria nem verificação automática de atualizações sem decisão posterior.

## 12. Git e documentação

- Inicializar repositório Git e criar `.gitignore` apropriado para Rust, Slint e IDEs.
- Ignorar `target/`, `dist/`, `*.AppDir/`, logs, temporários e artefatos locais.
- **Versionar `Cargo.lock`**.
- Criar `README.md` preferencialmente em inglês, com visão geral, recursos, limitações, instalação, compilação, capturas de tela futuras, contribuições e licença.
- Incluir `LICENSE` após verificação de compatibilidade das dependências.
- Preparar publicação no GitHub, sem presumir repositório remoto ou autenticação já existentes. Não executar publicação remota sem autorização.

## 13. GitHub Actions e distribuição

Configurar builds no GitHub, **sem exigir compilação local para publicar versões**.

**CI (push e pull request):**
- `cargo fmt --check`.
- `cargo clippy` com política de warnings definida.
- `cargo test`.
- Compilar a aplicação em runner Linux e Windows.
- Testar o crate `pdftrimly-core` independentemente.
- Validar traduções e artefatos quando viável.

**Release (tag `vX.Y.Z`):**
- Usar runner Windows para gerar o executável `pdftrimly.exe` e distribuir ZIP com recursos e runtimes necessários.
- Usar runner Ubuntu apropriado para gerar `.deb` com metadados, ícone, desktop entry, dependências de runtime e instalação no menu.
- Gerar `.AppImage` com linuxdeploy ou alternativa mantida, incluindo AppDir, recursos e dependências elegíveis.
- Verificar compatibilidade de glibc e bibliotecas; não supor que AppImage funcione em toda distribuição Linux.
- Publicar arquivos no GitHub Releases **somente após testes obrigatórios**.
- Usar permissões mínimas, versões confiáveis de actions e controle de integridade de downloads de ferramentas.

Nomes esperados:

```text
pdftrimly-v0.1.0-windows-x64.zip
pdftrimly-v0.1.0-linux-amd64.deb
pdftrimly-v0.1.0-linux-x86_64.AppImage
```

Disponibilizar artefatos de desenvolvimento quando apropriado. Testar instalação/execução em ambientes limpos, sem depender de bibliotecas acidentalmente presentes no runner.

## 14. Testes e benchmarks

Criar fixtures próprias ou de licença apropriada, incluindo:

- PDF somente texto e texto pesquisável.
- Digitalizações preto e branco e coloridas.
- Imagens JPEG, imagens Flate/PNG e transparências.
- Imagens CMYK e com máscaras, quando suportadas.
- Fotografias, vetores e conteúdo misto.
- Documentos multipágina, com links, marcadores, anotações, fontes e formulários.
- PDFs criptografados, assinados, malformados e grandes.
- Saídas existentes, caminhos duplicados e cancelamento.
- PDFs já otimizados ou cujo resultado seria maior.

Verificar: abertura estrutural, páginas, recursos essenciais, preservação de texto nos casos suportados, integridade dos originais, ausência de temporários abandonados, economia real quando aplicável e relatos corretos de falha. Considerar testes visuais comparativos em ambiente de teste com ferramentas externas, **sem torná-las dependências do motor de produção**.

Registrar tempo, tamanho original, tamanho final e economia por categoria. Não exigir porcentagem mínima universal.

## 15. Desempenho e robustez

- Não carregar todos os documentos da pasta simultaneamente.
- Processar um PDF por vez no MVP.
- Evitar buffers duplicados grandes e controlar a expansão de dados comprimidos.
- Manter GUI responsiva e cancelamento cooperativo.
- Registrar métricas de processamento e erros, sem conteúdo sensível.
- Evitar dependências pesadas sem justificativa.

## 16. Plano de execução e critérios de aceite

### Etapa 1 — Fundação
Criar workspace Rust, repositório Git, `.gitignore`, dependências verificadas, estrutura, README e licença viável. **Aceite:** `cargo check` e testes básicos passam.

### Etapa 2 — Prova de conceito do motor
Implementar análise e pelo menos uma otimização real; validar saídas e medir resultados em fixtures. **Aceite:** demonstração reproduzível de redução em casos elegíveis, sem alterações nos originais.

### Etapa 3 — Motor PDF
Implementar progressivamente os modos Leve, Normal e Ultra, tratamento de formatos elegíveis, validação e cancelamento. **Aceite:** testes por classe de PDF passam; limitações são explicitadas, sem simulação de recursos.

### Etapa 4 — Interface e idiomas
Criar UI Slint conforme referência, configurações, About, cinco idiomas, temas, diálogos e drag-and-drop suportado. **Aceite:** interface navegável, traduzida e redimensionável em Linux e Windows.

### Etapa 5 — Integração
Conectar motor à GUI; processar arquivos/pastas, progresso real, cancelamento, relatórios e tratamento de erros. **Aceite:** operação completa em lote, sem congelamento ou corrupção de saídas.

### Etapa 6 — Distribuição
Configurar CI, releases Windows ZIP/EXE, Linux DEB e AppImage. **Aceite:** builds reproduzíveis por tag, artefatos publicados e testes de instalação/execução documentados.

### Etapa 7 — Qualidade e lançamento
Revisar compatibilidade, traduções, acessibilidade, desempenho, licenças e documentação. **Aceite:** versão 0.1.0 pronta para distribuição, com limitações conhecidas claramente informadas.

## 17. Instruções ao agente de desenvolvimento

Antes de escrever código, analisar riscos técnicos, APIs, licenças, formatos PDF, limitações do `lopdf`, requisitos de empacotamento e critérios de aceite. Propor mudanças justificadas quando necessário.

**Executar em etapas, não implementar tudo de uma vez. Começar pela Etapa 1 e apresentar o resultado compilável, os comandos de verificação e o que falta.** Não criar botões fictícios, progresso simulado, compactação simulada ou declarações de sucesso sem validação. Em caso de recurso inviável, explicar a limitação e oferecer a alternativa tecnicamente correta.

**Objetivo final:** compactador PDF gratuito, elegante, confiável, offline e multiplataforma, simples para usuários comuns e sustentável para manutenção open source.
