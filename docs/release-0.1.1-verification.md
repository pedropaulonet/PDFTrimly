# Evidência local da preparação 0.1.1

Data: 11/10/2026. Estes resultados foram obtidos no host de desenvolvimento
Linux; não representam execução do GitHub Actions nem instalação em VM limpa.

| Verificação executada | Resultado |
| --- | --- |
| `cargo test --workspace --locked --offline` | 69 testes passaram; 0 falhas |
| `cargo build --release --locked --offline` | Build release concluído |
| `cargo fmt --all -- --check` | Passou |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 3 testes passaram: inventário ativo, textos aninhados e suplemento com proveniência |
| `desktop-file-validate assets/pdftrimly.desktop` | Passou |
| `git diff --check` | Passou |
| Metadata + árvore ativa Linux/Windows e execução local de `notices()` | 370/283 dependências externas, respectivamente; 0 textos ausentes após suplementos |
| `deb()` do script com o binário release 0.1.1 | DEB construído e inspecionado, extraído e executado: `pdftrimly 0.1.1` |

O DEB local declarou **`libc6 (>= 2.43)`**, calculado por `dpkg-shlibdeps`.
Não é evidência de compatibilidade com Ubuntu 24.04. O workflow usa Ubuntu
24.04 e precisa produzir seus próprios pacotes e dependências mínimas.
O DEB foi criado em uma pasta temporária; não foi instalado no host.

Não executados: workflow completo, build/execução Windows, empacotamento/execução
AppImage, instalação em sistemas limpos e roteiro gráfico nativo. `actionlint`
não estava disponível; a revisão do YAML foi estática. Não foi criada tag,
release ou rascunho remoto e nada foi publicado.

A auditoria de arquivos de licença não conclui a análise das obrigações do
runtime AppImage e das bibliotecas nativas. Consulte [dependencies.md](dependencies.md)
para os bloqueios de distribuição e [clean-install.md](clean-install.md) para
executar e registrar os ensaios restantes.
