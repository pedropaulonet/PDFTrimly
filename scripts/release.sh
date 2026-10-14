#!/usr/bin/env bash
set -Eeuo pipefail

die() { printf 'Erro: %s\n' "$*" >&2; exit 1; }
confirm() {
    local answer
    read -r -p "$1 [s/N] " answer
    [[ "$answer" == s || "$answer" == S || "$answer" == sim || "$answer" == SIM ]]
}
root=$(git rev-parse --show-toplevel 2>/dev/null) || die 'Execute dentro de um repositório Git.'
cd "$root"
git diff --quiet || die 'Há alterações não staged. Resolva-as antes de continuar.'
[[ -z $(git ls-files --others --exclude-standard) ]] || die 'Há arquivos não rastreados. Resolva-os antes de continuar.'

show_staged() {
    git diff --cached --check || die 'O diff staged contém erros de whitespace.'
    git diff --cached --stat
    git diff --cached --name-only
    git diff --cached
}
commit_staged() {
    git diff --cached --quiet && die 'Não há alterações staged.'
    show_staged
    local message
    read -r -p 'Mensagem do commit: ' message
    [[ -n "$message" ]] || die 'A mensagem do commit não pode ficar vazia.'
    confirm "Criar commit '$message'?" || die 'Operação cancelada.'
    git commit -m "$message"
}
push_branch() {
    local branch remote
    branch=$(git branch --show-current)
    remote=$(git remote | head -n1)
    [[ -n "$remote" ]] || die 'Nenhum remote Git configurado.'
    printf 'Será enviado o branch %s para %s.\n' "$branch" "$remote"
    git log -1 --oneline
    confirm 'Confirmar push normal (sem force)?' || die 'Operação cancelada.'
    git push "$remote" "$branch"
}
release_tag() {
    local branch version tag remote latest
    branch=$(git branch --show-current)
    [[ "$branch" == main ]] || die 'A preparação de release só é permitida no branch main.'
    command -v cargo >/dev/null || die 'Cargo não está instalado.'
    command -v gh >/dev/null || die 'GitHub CLI (gh) não está instalado.'
    gh auth status >/dev/null || die 'Autentique o GitHub CLI com gh auth login.'
    remote=$(git remote | head -n1)
    [[ -n "$remote" ]] || die 'Nenhum remote Git configurado.'
    read -r -p 'Versão (x.y.z ou x.y.z-prerelease): ' version
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || die 'Versão semver inválida.'
    tag="v$version"
    git show-ref --verify --quiet "refs/tags/$tag" && die "A tag local $tag já existe."
    local remote_tag
    remote_tag=$(git ls-remote --tags "$remote" "refs/tags/$tag") || die 'Não foi possível consultar as tags remotas.'
    [[ -z "$remote_tag" ]] || die "A tag remota $tag já existe."
    if git diff --cached --name-only -- Cargo.toml Cargo.lock | grep -q .; then
        die 'Faça commit ou remova do stage alterações em Cargo.toml/Cargo.lock antes do bump de versão.'
    fi
    show_staged
    latest=$(git describe --tags --abbrev=0 2>/dev/null || true)
    if [[ -n "$latest" ]]; then git log --oneline "$latest..HEAD"; else git log --oneline; fi
    printf '\nTag: %s\nCargo.toml e Cargo.lock serão atualizados para %s.\n' "$tag" "$version"
    printf 'O workflow existente gera as notas do GitHub e pode publicar uma prerelease 0.x após Linux e Windows passarem.\n'
    confirm 'Aplicar o bump e criar o commit de release?' || die 'Operação cancelada.'
    sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?\"$/version = \"$version\"/" Cargo.toml
    cargo check --workspace --offline >/dev/null
    cargo metadata --offline --no-deps --format-version 1 | python3 -c 'import json,sys; expected=sys.argv[1]; packages=json.load(sys.stdin)["packages"]; actual={p["name"]:p["version"] for p in packages if p["name"] in {"pdftrimly","pdftrimly-core"}}; sys.exit(0 if actual == {"pdftrimly":expected,"pdftrimly-core":expected} else 1)' "$version" || die 'A versão do Cargo não corresponde à tag planejada.'
    git add Cargo.toml Cargo.lock
    git diff --cached --check
    git commit -m "chore: release $version"
    git tag -a "$tag" -m "PDFTrimly $version"
    printf 'Destino: %s/main e tag %s\n' "$remote" "$tag"
    confirm "Enviar main para $remote?" || die 'Commit e tag locais criados; push cancelado.'
    git push "$remote" main
    confirm "Enviar a tag $tag para $remote? Isso pode publicar uma prerelease após CI." || die 'Branch enviado; a tag permanece local.'
    git push "$remote" "$tag"
}

while true; do
    printf '\nPDFTrimly release helper\n1) Commit locally\n2) Commit and push branch\n3) Prepare and push release tag\n4) Quit\n'
    read -r -p 'Escolha: ' choice
    case "$choice" in
        1) commit_staged; break ;;
        2) commit_staged; push_branch; break ;;
        3) release_tag; break ;;
        4) exit 0 ;;
        *) printf 'Opção inválida.\n' >&2 ;;
    esac
done
