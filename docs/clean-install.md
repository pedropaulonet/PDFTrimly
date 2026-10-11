# Verificação de instalação da 0.1.1 em sistemas limpos

Este é um roteiro a executar, não um relatório de aprovação. Use VMs descartáveis
x86_64 sem Rust, ferramentas de desenvolvimento ou versões anteriores do PDFTrimly.
Não use o computador de desenvolvimento como evidência de ambiente limpo.
Teste exatamente os artefatos da mesma execução de **Build packages**.

## Registro obrigatório

Para cada VM, registre: data, commit e URL/id da execução do Actions, nome e SHA-256
do ZIP e dos pacotes, versão/edição/build do SO, arquitetura, sessão gráfica
(X11/Wayland), comandos, saída, códigos de retorno e capturas da GUI. Registre
falhas e correções, sem substituir um resultado por expectativa. Não anexe PDFs
privados. Depois de corrigir um pacote, repita com o novo artefato.

Matriz mínima: Ubuntu 24.04 atualizado em X11 e Wayland (DEB e AppImage),
Windows 11 x64 atualizado (ZIP portátil). Distribuições Linux mais antigas e
outras versões Windows não são consideradas verificadas por esses ensaios.

## Linux: checksum e DEB

Baixe e extraia o ZIP Linux para uma pasta do usuário. Abra um terminal nessa
pasta, com uma sessão gráfica e acesso à rede para instalar dependências:

```sh
sha256sum --check SHA256SUMS
sudo apt update
sudo apt install ./pdftrimly_0.1.1_amd64.deb
command -v pdftrimly
pdftrimly --version
pdftrimly --help
ldd /usr/bin/pdftrimly
dpkg-query -W pdftrimly
test -f /usr/share/doc/pdftrimly/licenses/THIRD-PARTY-NOTICES.txt
pdftrimly
```

Exija todos os hashes corretos, versão `0.1.1`, nenhuma biblioteca `not found`,
janela utilizável e entrada PDFTrimly no menu. Encerre a janela e abra pelo menu.
Confira o Sobre e a atribuição Slint. Instale um backend de portal apropriado à
sessão (por exemplo `xdg-desktop-portal-gtk` no GNOME), se os seletores não abrirem;
registre essa necessidade. Repita os seletores em X11 e Wayland.

## Linux: AppImage sem o DEB instalado

Use outra VM/snapshot para evitar que dependências do DEB ocultem falhas do
AppImage. Dentro da pasta extraída:

```sh
sha256sum --check SHA256SUMS
chmod +x PDFTrimly-0.1.1-x86_64.AppImage
./PDFTrimly-0.1.1-x86_64.AppImage --version
./PDFTrimly-0.1.1-x86_64.AppImage
```

Se faltar FUSE, registre a falha e execute o caminho alternativo:

```sh
APPIMAGE_EXTRACT_AND_RUN=1 ./PDFTrimly-0.1.1-x86_64.AppImage --version
APPIMAGE_EXTRACT_AND_RUN=1 ./PDFTrimly-0.1.1-x86_64.AppImage
```

O AppImage ainda depende de glibc compatível, fontes, sessão gráfica,
`xdg-open` para abrir pastas e portal/backend para seletores. Registre qualquer
pacote adicional necessário. Para inspecionar os avisos internos sem FUSE:

```sh
./PDFTrimly-0.1.1-x86_64.AppImage --appimage-extract
find squashfs-root/usr/share/doc/pdftrimly -type f
```

## Windows: ZIP portátil

Baixe o ZIP Windows da mesma execução e extraia-o pelo Explorer para
`$HOME\PDFTrimly 0.1.1` (o espaço exercita caminhos comuns). Preserve toda a pasta.
Abra PowerShell nessa pasta:

```powershell
$ErrorActionPreference = 'Stop'
Get-Content .\SHA256SUMS | ForEach-Object {
    $expected, $relative = $_ -split '  ', 2
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $relative).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "Checksum incorreto: $relative" }
}
Test-Path .\LICENSE
Test-Path .\licenses\THIRD-PARTY-NOTICES.txt
.\pdftrimly.exe --version
if ($LASTEXITCODE -ne 0) { throw 'Falha no comando --version' }
.\pdftrimly.exe --help
if ($LASTEXITCODE -ne 0) { throw 'Falha no comando --help' }
Start-Process .\pdftrimly.exe -Wait
```

Exija `True` nas verificações de arquivos, versão `0.1.1` e janela utilizável.
Depois abra pelo Explorer sem privilégios de administrador. Registre avisos
SmartScreen/antivírus do executável sem assinatura; não desative proteções para
simular aprovação. Se a política bloquear a execução, registre como bloqueado.
Se faltar DLL, registre o nome e não copie DLLs do ambiente de desenvolvimento.

## Ensaio funcional comum

Use PDFs sintéticos ou públicos autorizados e registre seus hashes. O gerador
em [gui-verification.md](gui-verification.md) pode ser executado no ambiente de
build; transfira apenas os PDFs para a VM.

1. Processe um PDF na GUI e pela CLI (`compress entrada.pdf saida.pdf --level normal`).
2. Compare o hash do original antes/depois. Abra a saída em um leitor independente,
   compare as páginas e pesquise/copie texto de um fixture com OCR existente.
3. Teste seleção de vários PDFs, pasta com subpastas, caracteres acentuados,
   nomes longos, saída existente e pasta sem permissão de escrita.
4. Confira seletores, drop, copiar resumo e abrir destino. Em Wayland sem XWayland,
   drop não é suportado; confira seleção pelo portal e registre a limitação.
5. Salve idioma/tema/destino, encerre e reabra. Confira persistência, Sobre e links.
6. Pare e feche durante um lote: o arquivo atual pode terminar; não deve haver
   perda do original nem saída incompleta publicada.
7. Desconecte a rede e comprima novamente; links externos não fazem parte desse teste.

Resultado por cenário: **passou**, **falhou**, **bloqueado** ou **não executado**.
Aprovação exige evidência de instalação, CLI e GUI em cada combinação da matriz;
`--version` no runner de build não substitui esse ensaio.
