# Executáveis e pacotes

O workflow **Build packages** (`.github/workflows/packages.yml`) gera:

- Windows x86_64: `pdftrimly.exe`, portátil, com licenças ao lado.
- Linux x86_64: `pdftrimly_0.1.2_amd64.deb` e `PDFTrimly-0.1.2-x86_64.AppImage`.

A versão dos nomes vem do Cargo. O workflow roda ao enviar alterações para
`main`, enviar tags `v*` ou usar **Actions → Build packages → Run workflow**.
Abra a execução concluída e baixe **Artifacts**. O GitHub entrega um ZIP por
plataforma, retido por 30 dias.

Uma tag `v<versão>` publica a release após os dois builds e a
conferência de checksums. Versões 0.x são marcadas como pré-release. A versão da
tag precisa corresponder ao Cargo: os nomes Linux são conferidos ao montar
os arquivos. A release inclui DEB, AppImage, ZIP Windows completo, LICENSE e
SHA256SUMS; não oferece EXE avulso sem os avisos. Push para `main` e execução
manual geram somente artefatos. A publicação depende da execução bem-sucedida do workflow no GitHub.

Antes de enviar a tag, confira a [auditoria de dependências](dependencies.md)
e o [ensaio em ambientes limpos](clean-install.md). Reexecutar o job também anexa os pacotes a uma release já publicada; arquivos
com o mesmo nome são substituídos pelos artefatos daquela execução. O job usa `GITHUB_TOKEN`,
com escrita limitada ao job de publicação.

## Uso

No Windows, extraia o ZIP e abra `pdftrimly.exe`. Ele também aceita comandos
no terminal. O executável ainda não é assinado e não é um instalador.

No Debian/Ubuntu, extraia o ZIP e instale o DEB com:

```sh
sudo apt install ./pdftrimly_0.1.2_amd64.deb
```

O pacote registra o aplicativo no menu e instala o comando `pdftrimly`.
Dependências ELF são calculadas por `dpkg-shlibdeps`; bibliotecas carregadas
dinamicamente (Fontconfig, X11/Wayland e XKB), fontes e `xdg-utils` são declaradas
explicitamente. O build usa Ubuntu 24.04; Ubuntu 24.04 ou mais recente é a base
de compatibilidade, e o Debian precisa satisfazer as versões de bibliotecas
declaradas pelo pacote.

Para o AppImage:

```sh
chmod +x PDFTrimly-0.1.2-x86_64.AppImage
./PDFTrimly-0.1.2-x86_64.AppImage
```

Em sistemas sem FUSE, use:

```sh
APPIMAGE_EXTRACT_AND_RUN=1 ./PDFTrimly-0.1.2-x86_64.AppImage
```

O AppImage reúne bibliotecas nativas, incluindo as carregadas dinamicamente,
mas utiliza glibc, fontes e serviços do desktop do sistema. Não promete rodar
em distribuições mais antigas que a base de compilação. Diálogos precisam de
um portal do desktop; abrir pastas depende de `xdg-open`. Confira o roteiro
de operação gráfica em [linux-interface.md](linux-interface.md).

## Verificação e construção

Cada job executa os testes e compila com `cargo build --release --locked`.
`scripts/package.py` copia as licenças disponíveis nos crates resolvidos e
lista suas declarações e fontes em `licenses/THIRD-PARTY-NOTICES.txt`.
A árvore de `cargo tree --target` limita o inventário às dependências ativas,
incluindo build/test (um conjunto conservador, não uma lista de código ligado).
`NOTICE-AUDIT.json` registra textos ausentes. Diretórios `LICENSES/` também são
copiados; suplementos versionados podem completar textos omitidos pelos crates.
`SHA256SUMS` cobre todos os arquivos, incluindo avisos e suplementos.
O AppImage também inclui os arquivos de copyright Debian das bibliotecas
nativas reunidas. Isso apoia a revisão de distribuição descrita em
[dependencies.md](dependencies.md); não conclui essa revisão.

O linuxdeploy está fixado em `1-alpha-20251107-1`, com SHA-256 verificado pelo
script antes da execução. O runtime AppImage vem do asset oficial `596078161`,
também verificado por SHA-256; a ferramenta não baixa um runtime implícito.
Checkout e upload-artifact usam commits fixos.
`SHA256SUMS` registra os hashes dos arquivos gerados. Os pacotes Linux são
testados pelo comando `--version`, incluindo extração do DEB e execução do
AppImage sem FUSE. Instalação e operação gráfica em máquinas limpas continuam
a exigir verificação manual. Esses são passos configurados no workflow, não
resultados de uma execução conferida nesta revisão. A evidência local está em
[release-0.1.1-verification.md](release-0.1.1-verification.md).

Para reproduzir o empacotamento, use os comandos de build e metadata do
workflow, seguidos de:

```sh
cargo tree --locked --target x86_64-unknown-linux-gnu --prefix none --format '{p}' > dist/dependency-tree.txt
python3 scripts/package.py linux --metadata dist/metadata.json --dependency-tree dist/dependency-tree.txt --linuxdeploy dist/linuxdeploy.AppImage --runtime dist/appimage-runtime
```

No Windows, gere a árvore para `x86_64-pc-windows-msvc` e use o subcomando `windows`. Use um diretório de saída novo em cada
execução (`--output`).

`projeto.md` é um documento local ignorado pelo Git e não acompanha os pacotes.

## Limites de reprodutibilidade

Os comandos e as versões no lockfile tornam o procedimento repetível, mas não
há garantia de pacotes byte a byte idênticos: Rust `stable`, `windows-latest`,
pacotes APT e timestamps variam. Registre `rustc -Vv`, `cargo -V`, imagem do runner,
commit, dependências APT e hashes por execução. Os hashes das ferramentas AppImage
são conferidos pelo script; checksums dos pacotes identificam artefatos, não
atestam compatibilidade de instalação. Consulte [clean-install.md](clean-install.md).
