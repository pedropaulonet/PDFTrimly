# Executáveis e pacotes

O workflow **Build packages** (`.github/workflows/packages.yml`) gera:

- Windows x86_64: `pdftrimly.exe`, portátil, com licenças ao lado.
- Linux x86_64: `pdftrimly_0.1.0_amd64.deb` e `PDFTrimly-0.1.0-x86_64.AppImage`.

A versão dos nomes vem do Cargo. O workflow roda ao enviar alterações para
`main`, enviar tags `v*` ou usar **Actions → Build packages → Run workflow**.
Abra a execução concluída e baixe **Artifacts**. O GitHub entrega um ZIP por
plataforma, retido por 30 dias.

Ao enviar uma tag `v<versão>`, o workflow aguarda os dois builds, baixa os
artefatos da mesma execução, confere seus checksums e publica uma GitHub Release
com EXE, DEB, AppImage, ZIP do Windows com licenças, LICENSE e SHA256SUMS.
A versão da tag deve coincidir com a versão no Cargo; uma divergência impede
a publicação. Tags `v0.*` e versões com sufixo (por exemplo `v1.0.0-beta.1`)
são marcadas como pré-release. Versões estáveis a partir de 1.0.0 são releases
normais. Push de `main` gera apenas os artefatos.

Para publicar a versão atual, primeiro envie o commit que configura esse job:

```sh
git push origin main
git tag -a v0.1.0 -m "PDFTrimly 0.1.0"
git push origin v0.1.0
```

Acompanhe **Actions → Build packages**. A release aparece em **Releases** depois
que ambos os pacotes terminam. O job monta uma release em rascunho, anexa todos
os arquivos e então publica. Uma reexecução pode completar um rascunho, mas não
substitui uma release já publicada. Não é necessário configurar um token pessoal;
o job usa `GITHUB_TOKEN` com permissão de escrita apenas na publicação.

## Uso

No Windows, extraia o ZIP e abra `pdftrimly.exe`. Ele também aceita comandos
no terminal. O executável ainda não é assinado e não é um instalador.

No Debian/Ubuntu, extraia o ZIP e instale o DEB com:

```sh
sudo apt install ./pdftrimly_0.1.0_amd64.deb
```

O pacote registra o aplicativo no menu e instala o comando `pdftrimly`.
Dependências ELF são calculadas por `dpkg-shlibdeps`; bibliotecas carregadas
dinamicamente (Fontconfig, X11/Wayland e XKB), fontes e `xdg-utils` são declaradas
explicitamente. O build usa Ubuntu 24.04; Ubuntu 24.04 ou mais recente é a base
de compatibilidade, e o Debian precisa satisfazer as versões de bibliotecas
declaradas pelo pacote.

Para o AppImage:

```sh
chmod +x PDFTrimly-0.1.0-x86_64.AppImage
./PDFTrimly-0.1.0-x86_64.AppImage
```

Em sistemas sem FUSE, use:

```sh
APPIMAGE_EXTRACT_AND_RUN=1 ./PDFTrimly-0.1.0-x86_64.AppImage
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
a exigir verificação manual.

Para reproduzir o empacotamento, use os comandos de build e metadata do
workflow, seguidos de:

```sh
python3 scripts/package.py linux --metadata dist/metadata.json --linuxdeploy dist/linuxdeploy.AppImage --runtime dist/appimage-runtime
```

No Windows, use o subcomando `windows`. Use um diretório de saída novo em cada
execução (`--output`).

`projeto.md` é um documento local ignorado pelo Git e não acompanha os pacotes.
