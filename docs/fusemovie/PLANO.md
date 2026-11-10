# Fusemovie: plano a partir do FilmCraft

Fork de `storytold/filmcraft` (espelha o Adobe Premiere Pro), seguindo o roteiro da família Fuse em
`fernandinhomartins40/fuseopenphoto`, `docs/fuse-suite/` (README, 00-playbook, 01-codex, 02-studio-design,
03-build-release, 04-licoes e `apps/filmcraft.md`). Nome do produto: **Fusemovie** (no guia era
"Fuse Open Film").

Levantamento de 2026-11-10, sobre o upstream `f4090d7b` (v0.17.0).

## Fase 1: feita

- `origin` = `fernandinhomartins40/fusemovie` (público). `upstream` = `storytold/filmcraft`, push `DISABLED`.
- Branch de trabalho e padrão do repositório: `studio-redesign`, com o histórico completo do upstream.
- Só `studio-redesign` foi enviado. Os workflows do upstream disparam em `main`/`release`, então nenhum
  rodou. Exceção: `windows-arm64.yml` tem `schedule` noturno, que roda no branch padrão e vai falhar até a
  Fase 2 trocar os workflows.

## O que o upstream tem (e o que muda por isso)

| Ponto | No FilmCraft | Consequência para o Fusemovie |
|---|---|---|
| Interface | egui 0.37, fork `storytold/egui` rev `6d518fc`, igual ao Photo | As lições de `04-licoes.md` valem diretamente |
| Crates | 51 em `crates/`, apps `filmcraft`, `filmcraft-cli`, `filmcraft-web` | Nomes de crate para os workflows: `filmcraft`, `filmcraft-cli`, `filmcraft-ui-egui` |
| Workflows | `ci`, `house-rules`, `nix`, `packaging-lint`, `release`, `sonarcloud`, `freebsd`, `windows-arm64` (dependem de `craft-common-actions`) | Apagar todos; copiar `build-windows.yml` e `checks.yml` do Photo |
| Empacotamento | `packaging/windows/package.ps1`, `filmcraft.wxs`, `install-policy.*`, `sign.ps1` | Adaptar o `package.ps1` para ler o `brand.env` e incluir `codex*.exe` |
| Marca | `crates/ui-egui/src/brand.rs` desenha o wordmark ArtCraft (`docs/brand/`, licença fechada); títulos "FilmCraft" fixos em `apps/filmcraft/src/main.rs` e `native_menu.rs`; `APP_ID` `ai.storyteller.filmcraft` | `brand.env` + `build.rs`; trocar wordmark por ícone original; títulos e id pelo `brand.env` |
| Ícone | `assets/app-icon/filmcraft.*` (png, svg, ico, icns) | Ícone original Fusemovie (fundo âmbar), regenerar todos os formatos e `ATTRIBUTION.md` |
| Atualizações | `crates/ui-egui/src/updates_ui.rs`, checagem na inicialização e no menu, aponta para `storytold/filmcraft/releases` | Desligar com teste, como no Photo (`ce9a354f1`) |
| i18n | `crates/ui-egui/src/i18n/*.tsv`, já tem `pt-br.tsv` | Nome do produto vira `{app}`; strings novas ganham linha pt-BR |
| Canal de controle | `filmcraft --control PORTA`; **o token `FILMCRAFT_CONTROL_TOKEN` (64 hex) é obrigatório também no app**, sem ele o canal fica fechado | O serviço do Codex gera o token, põe no ambiente do app antes de abrir o canal e passa ao MCP por `env_vars` |
| MCP | `filmcraft-cli mcp --bridge 127.0.0.1:PORTA` (confere com o guia) | Argumentos do `codex app-server` como no Photo, com `default_tools_approval_mode="approve"` |
| `render_preview` | Em bridge é um print do monitor Program (`ui.screenshot` de painel) | Fazer renderizar o quadro da sequência pelo motor, com `region`, como no Photo |
| Layout | Painéis em docking estilo Premiere (`dock.rs`, `panels/timeline/`, `monitor.rs`), sem Studio | Fase 5 inteira; reaproveitar `crates/ui-egui/src/studio/` do Photo (~6 mil linhas com o Codex) |

## Próximas fases

2. **Build e instalador**: trocar os workflows, `package.ps1` com `brand.env` e Codex CLI, pré-release
   `latest-studio-redesign`. Só depois disso dá para testar qualquer coisa.
3. **Marca**: `brand.env`, `{app}` nos catálogos, ícone original, sem wordmark ArtCraft, sem links nem
   atualizações da storytold, dados em diretório próprio.
4. **Codex**: `codex.rs`, `codex_panel.rs` e o serviço do Photo, com o token obrigatório acima.
5. **Studio**: modos Montar · Cortar · Cor · Áudio · Legendas; faixa inferior = linha do tempo (trilhas de
   vídeo e áudio); ferramentas rápidas Seleção, Navalha, Ondulação, Deslizar, Caneta, Texto, Zoom, Mão.
   Precisa dos mockups do dono em `docs/redesign/` antes do código.
6. **Compacto e acabamento.**
7. **Montagem automática a partir de um roteiro** (pasta de clipes + briefing → primeiro corte, legendas
   pelo crate `speech`, títulos editáveis, cor por ajustes). Cursor animado vai à posição na linha do
   tempo; análise só por quadros-chave.

## Pendências com o dono

- Grafia final do nome (`Fusemovie`, `FuseMovie` ou `Fuse Movie`) e o `BRAND_ID` (proposta:
  `BRAND_NAME=Fusemovie`, `BRAND_COMPANY=Fuse`, `BRAND_ID=com.fuse.fusemovie`, `BRAND_FILE=fusemovie`).
- Mockups das 7 telas do Studio com conteúdo de vídeo.
