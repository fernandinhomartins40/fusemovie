# Fusemovie: plano

Fork de `storytold/filmcraft` (espelha o Adobe Premiere Pro), seguindo o roteiro da família Fuse em
`C:/FuseDev/fuseopenphoto/docs/fuse-suite/` (README, 00-playbook, 01-codex, 02-studio-design,
03-build-release, 04-licoes e `apps/filmcraft.md`). Nome do produto: **Fusemovie**.

Atualizado em 2026-11-11 sobre o commit `39018254` do branch `studio-redesign`. Este arquivo diz o que
falta e em que ordem. Como lançar os agentes está em [`ORQUESTRACAO.md`](ORQUESTRACAO.md).

## 1. Onde estamos

Feito, integrado e com instalador publicado do commit `39018254` (checks e build verdes):

| Fase | Estado | Onde está |
|---|---|---|
| 1 Fork | feito | `origin` = `fernandinhomartins40/fusemovie`, `upstream` só leitura, branch padrão `studio-redesign` |
| 2 Build | feito | `.github/workflows/build-windows.yml` (MSI + zip, pré-release `latest-studio-redesign`), `checks.yml` |
| 3 Marca | feito | `brand.env`, `crates/ui-egui/src/brand.rs`, catálogos com `{app}`, ícone novo, sem atualizações |
| 4 Codex | feito, **não testado de verdade** | `apps/filmcraft/src/codex.rs`, `crates/ui-egui/src/codex_panel.rs`, `codex_service` em `apps/filmcraft/src/main.rs` |
| 5 Studio | feito, **não visto no app real** | `crates/ui-egui/src/studio/` (`mod`, `top_bar`, `mode_bar`, `tools`, `right`, `bottom`, `home`, `new_doc`, `style`, `tests`) |
| 6 Compacto e acabamento | só o básico | tamanhos compactos existem e têm teste; nada foi ajustado com print do dono |
| 7 Montagem automática | **não começou** | "Criar com IA" na Início está desativado ("em breve") |

Telas geradas pelos testes: `docs/redesign/fusemovie/*.png`.

## 2. O que a análise encontrou

Em ordem de importância. Cada item diz em que onda é resolvido.

1. **O instalador não transcreve fala.** A transcrição (Whisper) é a feature `whisper` do Cargo, desligada
   por padrão, e o `packaging/windows/package.ps1` compila sem ela. Sem isso, `transcript.generate` fica
   desativado e a função de destaque não faz legendas. *Onda 1, tarefa T1.*
2. **O Codex não consegue ver um clipe antes de colocá-lo na linha do tempo.** O `render_preview` só
   renderiza a sequência ativa. A montagem automática precisa olhar quadros de cada clipe da pasta.
   *Onda 1, T1.*
3. **A suíte inteira de testes nunca rodou no fork.** O `checks.yml` roda só os testes com `studio`,
   `home`, `new_doc`, `codex`, `brand` no nome. O agente do Studio trocou `FilmcraftApp::new` por
   `new_classic` em 105 arquivos de `crates/ui-egui/tests/it/` e esses testes não foram executados.
   Os portões `cargo xtask assets`, `layers` e `wasm` também não. *Onda 0 dispara; Onda 1, T3 corrige.*
4. **O painel do Codex não tem os ganchos que o "Criar com IA" usa.** No Photo, o `create_ai.rs` chama
   `ensure_started`, `new_chat`, `send_as`, `switch_model`, `has_model`, `last_error`,
   `last_turn_error`, `limit_reached`, `is_signed_in`, `strip_tag`. No Fusemovie só existem `is_busy`,
   `is_running`, `take_drop`. *Onda 1, T2.*
5. **O `AGENTS.md` contradiz o fork.** Ele manda o CI chamar `storytold/craft-common-actions`, abrir PR
   para `main` e rodar `cargo xtask ci` antes de cada commit. Todo agente carrega esse arquivo e gasta
   tokens resolvendo a contradição. *Onda 0.*
6. **Todo merge deu conflito no `pt-br.tsv`**, porque cada agente acrescenta linhas no fim. *Onda 0.*
7. **Sobras pequenas:** 2 marcas `// integrator: switch to theme::Tokens` em `studio/home.rs`; 11
   textos com "FilmCraft" fixo em `crates/ui-egui/src` e `apps/filmcraft/src`; 1 linha no `pt-br.tsv`;
   README ainda é o do upstream. *Onda 1, T4.*
8. **Nada foi aberto de verdade.** Login, Codex real, GPU e reprodução só o dono testa. *Portão do dono.*

Boas notícias que encurtam o trabalho:

- A linha do tempo já tem todos os comandos que a receita precisa: `file.import`, `file.newSequence`,
  `timeline.place`, `timeline.razor`, `timeline.trim`, `timeline.move`, `sequence.applyVideoTransition`,
  `graphics.newText`, `lumetri.setSection`, `lumetri.applyMatch`, `clip.audioGain`,
  `transcript.generate`, `transcript.inspect`, `transcript.createCaptions`, `transcript.models`,
  `transcript.downloadModel` (ver `docs/transcripts.md`).
- O `drain_control` (`crates/ui-egui/src/app/control.rs`) já tem a fila `deferred`: dá para segurar
  0,12 s entre edições sem mudar o protocolo.
- O `ui.renderPreview` já renderiza o quadro pelo motor e aceita `region`.

## 3. O que falta, em ondas

| Onda | Quem faz | O que entrega | Depende de |
|---|---|---|---|
| 0 | o orquestrador, sem agentes | limpeza das worktrees antigas, regras do fork no `AGENTS.md`, catálogos sem conflito de merge, workflow `full-tests` disparado | nada |
| 1 · T1 | agente Sonnet | transcrição no instalador (feature `whisper`) e `render_preview` de um clipe do projeto (`item`) | Onda 0 |
| 1 · T2 | agente Opus | "Criar com IA": diálogo, projeto e sequência criados pelo app, editor travado com etapas, cursor na linha do tempo, pausa no limite e retomada, a receita | Onda 0 |
| 1 · T4 | agente Sonnet | Início e Novo projeto nas cores do tema, os 11 textos "FilmCraft" restantes, README do Fusemovie | Onda 0 |
| 1 · T3 | agente Sonnet, só se precisar | correção do que a suíte inteira de testes acusar | resultado do `full-tests` |
| integração | o orquestrador | merge, clippy, um push, instalador publicado do commit final | T1, T2, T4 (e T3) |
| portão | **o dono** | instalar e testar de verdade (lista na seção 4) | instalador publicado |
| 2 | um agente Sonnet e o orquestrador | acabamento do modo compacto pelos prints do dono; ajuste da receita e do Codex pelo que o teste real mostrar | portão |

T1, T2 e T4 rodam ao mesmo tempo: os arquivos de cada uma não se cruzam. São no máximo 3 agentes por vez,
porque na rodada anterior 4 agentes bateram o limite de uso duas vezes.

## 4. Portão do dono, e a Onda 2

Só depois que o instalador com T1 a T4 estiver publicado. Não lance agentes de acabamento antes: sem o
teste real eles ajustam por suposição.

O dono instala e confere, nesta ordem, mandando print do que falhar:

1. O app abre na Início; Novo projeto cria a sequência; arrastar um vídeo funciona; o vídeo toca.
2. Os 5 modos, as ferramentas e a linha do tempo no notebook (modo compacto).
3. Varinha › entrar com ChatGPT › pedir "importe estes clipes e monte em sequência". O status deve
   chegar a "MCP conectado".
4. Criar com IA com 3 a 5 clipes curtos e um briefing de duas linhas.

Onda 2, conforme o que voltar:

- **Um agente Sonnet de acabamento**, com todos os prints do dono no mesmo prompt (fase 6: compacto,
  textos cortados, rolagem). Um agente só, porque os ajustes caem nos mesmos arquivos de `studio/`.
- **Receita:** o que o Codex real errar (comando com parâmetro errado, etapa pulada) corrige-se no
  texto da receita e na descrição das ferramentas MCP. É edição de texto: o orquestrador faz sozinho.
- **Erros do Codex real** (login, "MCP tool call requires approval", porta, token): o orquestrador lê
  o log que o dono mandar e corrige direto em `apps/filmcraft/src/codex.rs`; só lança agente se
  passar de um arquivo.

Pronto, para dizer ao dono: instalador da pré-release no commit final, checks verdes, `full-tests` sem
falha nossa, e a lista do que ele testou e do que continua sem teste real.

## 5. Quando está pronto

- Pré-release `latest-studio-redesign` publicada do último commit com código, com `Checks` e
  `Build Windows installer` verdes.
- `full-tests` sem falha nossa (as de fonte CJK não contam: o CI não tem as fontes).
- O dono abriu o app, entrou no Codex e rodou um "Criar com IA" de ponta a ponta.
- O que continuar sem teste real é dito ao dono, item por item.

## 6. Fora deste plano

Ficam para depois, por decisão de escopo: assinatura de código do instalador (custa dinheiro; decisão
do dono), abrir `.prproj`, vozes neurais (`neural-voices`), datas na lista de conversas do Codex, coluna
do Codex redimensionável no layout clássico, builds para macOS e Linux.
