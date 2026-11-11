# Fusemovie: plano de orquestração para terminar a implementação

Para a sessão que vai orquestrar os agentes (Opus 5.5) e para o dono acompanhar. Escrito em
2026-11-11 sobre o commit `39018254` do branch `studio-redesign`. Substitui a parte "Próximas fases" do
[`PLANO.md`](PLANO.md). O roteiro geral da família continua em `C:\FuseDev\fuseopenphoto\docs\fuse-suite\`.

Leia este arquivo inteiro uma vez. Depois siga as ondas na ordem; cada onda traz os prompts prontos.

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

## 3. Por que a rodada anterior saiu cara, e as regras para esta

Medido na rodada de 2026-11-10: cada agente terminou com 250 a 330 mil tokens de contexto, os 4 bateram
o limite de 5 horas duas vezes, e cada um acordou de 5 a 10 vezes só para ver se o CI tinha terminado.
Cada despertar relê o contexto inteiro.

Regras desta rodada:

1. **Agente não espera CI.** Ele faz, roda `cargo clippy` local, faz commit, faz push do branch dele e
   **termina** com um relatório curto. Quem espera o CI é o orquestrador, com o laço da seção 9, que
   não gasta tokens. Se falhar, o orquestrador manda ao mesmo agente (SendMessage) só o trecho do log.
2. **No máximo 3 agentes ao mesmo tempo.**
3. **Prompt curto.** As regras ficam no `AGENTS.md` (Onda 0); o prompt traz só worktree, tarefa,
   arquivos e critério de pronto. Não repita o roteiro.
4. **Sem exploração.** Os prompts abaixo já dão os arquivos e os nomes. Há arquivos de 80 a 130 KB
   (`menus.rs`, `effect_controls.rs`, `monitor.rs`): usar Grep e ler trechos, nunca o arquivo inteiro.
5. **Modelo por tarefa.** Sonnet 5.5 para o mecânico (T1, T3, T4); Opus 5.5 só para T2.
6. **Um push por onda no `studio-redesign`.** Cada push cancela o build anterior
   (`cancel-in-progress`). Junte os merges e envie uma vez.
7. **Não regenerar os PNGs de `docs/redesign/fusemovie/`** se a tela não mudou. Cada um exige um
   arquivo `.attribution` e uma linha no `ATTRIBUTION.md`.
8. **Limite de uso:** se um agente cair, salve o trabalho dele com o comando da seção 9 e retome o
   mesmo agente com SendMessage. Não lance outro no lugar.
9. **Disco:** 33 GB livres. Cada `CARGO_TARGET_DIR` ocupa uns 7 GB. Apague o de cada agente depois do
   merge.
10. **`isolation: "worktree"` da ferramenta Agent não funciona nesta máquina** (confunde `c:` com
    `C:`). Crie a worktree à mão (seção 9) e passe o caminho no prompt.

Regras do dono, que continuam valendo: nunca abrir ou automatizar navegador nem disparar login; `gh`
sempre com `-R fernandinhomartins40/fusemovie`; consultar o GitHub no máximo a cada 5 minutos; só dizer
"pronto" com a pré-release `latest-studio-redesign` publicada do commit final.

## 4. Onda 0: o orquestrador sozinho (sem agentes)

Tudo aqui é pequeno e mecânico. Faça direto no `studio-redesign`, em um commit, e um push no fim.

1. **Limpeza.** As 4 worktrees antigas já foram integradas:
   `git worktree remove` em cada uma de `.claude/worktrees/agent-*`, `git branch -D` nos 4 branches
   `worktree-agent-a*`, e apague `C:\FuseDev\target-codex` se o disco apertar. Os branches remotos e o
   PR #1 do Dependabot ficam para o dono apagar (a sessão anterior não teve permissão).
2. **`.gitattributes`**, para os catálogos deixarem de dar conflito:
   ```
   crates/ui-egui/src/i18n/*.tsv merge=union
   ATTRIBUTION.md merge=union
   ```
3. **`AGENTS.md`**: acrescente no topo, antes de "House rules", a seção abaixo.
   ```markdown
   ## Fusemovie (este fork): vale acima do resto deste arquivo

   Este repositório é o Fusemovie, fork de storytold/filmcraft. Onde esta seção e o resto do arquivo
   discordam, esta seção vale.

   - Branch de trabalho e padrão: `studio-redesign`. Não há PR para `main`. Agentes trabalham em
     branches `worktree-agent-<nome>` e não fazem push em `studio-redesign`.
   - CI próprio: `.github/workflows/checks.yml` e `build-windows.yml`. Não usar
     `storytold/craft-common-actions`. Não rodar `cargo xtask ci` inteiro; rodar
     `cargo clippy -p <crates tocados> --all-targets -- -D warnings` antes de cada commit.
   - Nunca abrir, automatizar ou emular navegador, nem disparar login (Codex, ChatGPT, Google).
     Verificação visual só por testes kittest fora da tela.
   - `gh` sempre com `-R fernandinhomartins40/fusemovie`. Consultar o GitHub no máximo a cada 5 minutos.
   - Não esperar o CI: commit, push do próprio branch e terminar com o relatório.
   - Nome do produto: nunca escrever "FilmCraft" em texto que o usuário vê; usar `{app}` nos
     catálogos e `crate::brand::APP_NAME` no código. Nomes de crate, `.fcproj` e variáveis
     `FILMCRAFT_*` não mudam.
   - Toda string nova passa pelo `tl!` e ganha linha em `crates/ui-egui/src/i18n/pt-br.tsv` no mesmo
     commit. Cores só por `theme::Tokens`.
   - Testes do layout antigo usam `FilmcraftApp::new_classic`; o padrão (`new`) é o Studio.
   - As marcas ArtCraft de `docs/brand/` não podem ser usadas (fork).
   - Armadilhas do egui deste fork: `ctx.data` dentro de `ctx.data_mut` trava o app; usar
     `Id::unique`, não `Id::new`; `ScrollArea::horizontal` não converte a roda vertical.
   - Não ler arquivos grandes inteiros (`menus.rs`, `effect_controls.rs`, `monitor.rs`): Grep e trechos.
   ```
4. **`.github/workflows/full-tests.yml`** (manual, não roda em push):
   ```yaml
   name: Full tests
   on: workflow_dispatch
   jobs:
     full:
       runs-on: windows-latest
       timeout-minutes: 240
       env:
         CARGO_PROFILE_DEV_DEBUG: "0"
         CARGO_INCREMENTAL: "0"
       defaults:
         run:
           shell: bash
       steps:
         - uses: actions/checkout@v4
           with:
             persist-credentials: false
         - uses: dtolnay/rust-toolchain@stable
           with:
             targets: wasm32-unknown-unknown
         - uses: Swatinem/rust-cache@v2
           with:
             shared-key: checks
             cache-on-failure: true
         - name: Assets
           if: always()
           run: cargo xtask assets 2>&1 | tee assets.log
         - name: Layers
           if: always()
           run: cargo xtask layers 2>&1 | tee layers.log
         - name: UI tests (lib and integration)
           if: always()
           run: set -o pipefail; cargo test -p filmcraft-ui-egui --no-fail-fast 2>&1 | tee ui.log
         - name: Engine, automation, app, cli
           if: always()
           run: set -o pipefail; cargo test -p filmcraft-engine -p filmcraft-automation -p filmcraft -p filmcraft-cli --no-fail-fast 2>&1 | tee rest.log
         - name: Web build
           if: always()
           run: cargo xtask wasm 2>&1 | tee wasm.log
         - uses: actions/upload-artifact@v4
           if: always()
           with:
             name: full-test-logs
             path: "*.log"
   ```
5. Nos dois workflows de push (`checks.yml`, `build-windows.yml`), acrescente em `on.push`:
   `paths-ignore: ['docs/**', '**.md']`, para commit só de documentação não gastar 25 minutos de build.
   Atenção: depois disso, um commit só de docs não atualiza a pré-release; o "commit certo" passa a
   ser o último commit com código.
6. Commit, push, e dispare: `gh workflow run full-tests.yml -R fernandinhomartins40/fusemovie --ref studio-redesign`.
   Não espere: siga para a Onda 1. O resultado alimenta a tarefa T3.

## 5. Onda 1: três agentes em paralelo (T1, T2, T4), depois T3

Crie as worktrees (seção 9) e lance T1, T2 e T4 juntos. T3 só é lançado quando o `full-tests` terminar,
e só se houver falha nossa.

Os arquivos de cada tarefa não se cruzam, fora os catálogos (resolvidos pelo `merge=union`):

| Tarefa | Modelo | Arquivos que pode tocar |
|---|---|---|
| T1 Ferramentas | Sonnet | `packaging/windows/package.ps1`, `crates/automation/src/server.rs` e testes, `crates/ui-egui/src/control.rs`, `docs/control-protocol.md`, `docs/agents.md` |
| T2 Criar com IA | Opus | novo `crates/ui-egui/src/create_ai.rs`, `codex_panel.rs`, `app/control.rs`, `app/frame.rs`, `menus.rs` (um braço), `lib.rs` (um campo), `studio/home.rs` (só o item "Criar com IA") |
| T4 Acabamento | Sonnet | `studio/home.rs` (só a paleta), os 11 textos, `README.md`, `docs/fusemovie/PLANO.md` |
| T3 Suíte inteira | Sonnet | testes em `crates/ui-egui/tests/`, sidecars de assets; código só se o teste provar um defeito |

T2 e T4 tocam `studio/home.rs` em trechos diferentes (T2: a função do trilho, linha ~187; T4: as
constantes de paleta no topo). Faça o merge de T4 antes do de T2.

### T1: ferramentas que a montagem automática precisa (Sonnet)

```
Worktree: C:\FuseDev\fusemovie\.claude\worktrees\t1 (branch worktree-agent-t1). Trabalhe só nela;
todo comando começa com `cd /c/FuseDev/fusemovie/.claude/worktrees/t1 &&`.
CARGO_TARGET_DIR=C:\FuseDev\target-t1. As regras estão no topo do AGENTS.md (seção Fusemovie).

Tarefa, três partes:

1. Transcrição no instalador. Em packaging/windows/package.ps1, o `cargo build --release` passa a
   usar `--features filmcraft/whisper,filmcraft-cli/whisper` (a feature está em
   apps/filmcraft/Cargo.toml e apps/filmcraft-cli/Cargo.toml; ver docs/transcripts.md).

2. Ver um clipe sem colocá-lo na sequência. O método de controle `ui.renderPreview`
   (crates/ui-egui/src/control.rs) e a ferramenta MCP `render_preview`
   (crates/automation/src/server.rs, struct PreviewParams, modos bridge e headless) ganham o
   parâmetro opcional `item`: o id de um item de mídia do projeto. Com `item`, renderiza o quadro
   desse item em `seconds` (tempo da mídia, padrão 0), no tamanho da mídia reduzido a `max_side`
   (padrão 512 quando há `item`), aceitando `region` como hoje, sem mexer em sequência, playhead ou
   monitor Source. Descubra como o monitor Source decodifica um quadro (Grep por `source` em
   crates/engine/src/lib.rs e crates/ui-egui/src/source_playback.rs) e reutilize; não escreva um
   decodificador. Entradas hostis devolvem erro, nunca pânico: item inexistente, item sem vídeo,
   seconds negativo, NaN ou além da duração, region fora do quadro.
   Atualize a descrição da ferramenta MCP (o modelo lê essa descrição) e as linhas de
   docs/control-protocol.md e docs/agents.md.

3. Testes, com nomes contendo `codex` ou no crate automation (o CI filtra por nome): item da
   filmagem de demonstração renderiza com o tamanho certo; region com item; cada entrada hostil
   devolve erro; no modo bridge a ferramenta envia `item` no `ui.renderPreview`.

Pronto quando: `cargo clippy -p filmcraft-ui-egui -p filmcraft-automation --all-targets -- -D warnings`
passa, tudo está em commit e o branch foi enviado (`git push -u origin worktree-agent-t1`).
Não espere o CI. Termine dizendo: commits, o que rodou localmente, o que não foi testado.
```

Depois do merge, o orquestrador confere no build: o tempo (a feature compila o `candle`) e o tamanho
do MSI. Se o build passar de 150 minutos ou falhar por causa da feature, reverta só a parte 1 e avise
o dono; a parte 2 não depende dela.

### T2: "Criar com IA" = montagem automática a partir de um roteiro (Opus)

```
Worktree: C:\FuseDev\fusemovie\.claude\worktrees\t2 (branch worktree-agent-t2). Trabalhe só nela;
todo comando começa com `cd /c/FuseDev/fusemovie/.claude/worktrees/t2 &&`.
CARGO_TARGET_DIR=C:\FuseDev\target-t2. As regras estão no topo do AGENTS.md (seção Fusemovie).

Tarefa: portar a função "Criar com IA" do Fuse Open Photo para vídeo. A referência é
C:\FuseDev\fuseopenphoto\crates\ui-egui\src\create_ai.rs (913 linhas; leia inteiro, é o molde) e os
pontos onde o Photo o chama: app/control.rs (hold, observe), app/frame.rs (filter_keys, take_drop,
show), menus/invoke.rs (COMMAND), codex_panel.rs (Act::CreateAi, strip_tag, chip).
Leia também docs/fusemovie/ORQUESTRACAO.md, seção 6 (a receita e as decisões já tomadas).

Parte A, ganchos no painel (crates/ui-egui/src/codex_panel.rs). Faltam os que o create_ai usa;
copie do codex_panel.rs do Photo com as mesmas assinaturas: ensure_started, new_chat, send_as
(mensagem mostrada diferente da enviada, com anexos), switch_model, has_model, last_error,
last_turn_error, limit_reached, is_signed_in, strip_tag (esconde `[[n]]` do texto).

Parte B, novo crates/ui-egui/src/create_ai.rs, mesma estrutura do Photo (CreateAi, Dialog, Stage,
Run, Cursor; open, take_drop, filter_keys, hold, observe, tick, show), com estas diferenças:
- Diálogo: pasta de clipes ou arquivos soltos (arrastar ou escolher), briefing em texto de várias
  linhas (obrigatório), vídeo de referência opcional, trilha opcional, modelo, aviso de tempo e
  consumo, botão âmbar com a varinha. Comando `file.createWithAi`; liga o item "Criar com IA" da
  Início (crates/ui-egui/src/studio/home.rs, função do trilho, hoje desativado com "Coming soon").
- Ao iniciar: sair do layout clássico; `file.newProject`; `file.import` de todo o material;
  `file.newSequence` vazia chamada "Montagem" no formato do primeiro clipe com vídeo (veja como
  studio/new_doc.rs e studio/home.rs chamam esses comandos). O material fica no bin do projeto: é a
  "referência"; nada de camada oculta.
- Fala: antes de enviar a receita, consulte `transcript.models`. Sem modelo instalado, rode
  `transcript.downloadModel` mostrando "Baixando o modelo de fala". Se a transcrição não existir
  neste build, siga sem ela e use a variante da receita sem legendas.
- Durante o processo: editor travado (só o painel do Codex e a barra da janela respondem), botão
  Parar, aba inferior forçada em Linha do tempo, etapas pela marca `[[n]]`, 0,12 s entre edições
  usando a fila `deferred` que já existe em app/control.rs.
- Cursor âmbar: vai à POSIÇÃO NA LINHA DO TEMPO da edição, não ao monitor. Tire tempo e trilha dos
  parâmetros do comando (Grep em crates/engine/src pelos parâmetros de `timeline.place`,
  `timeline.razor`, `timeline.trim`, `timeline.move`, `graphics.newText`) e converta para a tela com
  as funções de tempo→x e trilha→y do painel (crates/ui-egui/src/panels/timeline/). Sem posição nos
  parâmetros: o playhead. Etiquetas por área do comando: Importando, Colocando um clipe, Cortando,
  Ajustando, Transição, Escrevendo o título, Legendas, Ajustando a cor, Ajustando o áudio.
- Limite de uso: pausa; outra conta retoma com a receita de continuação.
- Fim: Salvar (.fcproj, `file.saveAs`); parado no meio: Manter ou Descartar.
- A receita está em docs/fusemovie/ORQUESTRACAO.md, seção 6. Use aquele texto; confirme cada id de
  comando com um Grep em crates/engine/src antes de fixá-lo.

Testes (nomes contendo `codex` ou `studio`, o CI filtra por nome), kittest fora da tela e unitários:
diálogo exige briefing e material; iniciar cria projeto, importa e cria a sequência "Montagem";
marca `[[3]]` vira "Etapa 3 de 6" e some do texto; hold segura a próxima edição; observe põe o
cursor dentro do retângulo da linha do tempo; teclado bloqueado durante o processo e Esc fecha o
diálogo; limite pausa e outra conta retoma; Parar leva a Manter/Descartar.

Fora do escopo: ajustar a receita com o Codex real (isso é do dono), fase 6, snapshots PNG.
Faça commit a cada parte. Pronto quando: clippy de filmcraft-ui-egui e filmcraft com -D warnings
passa, tudo em commit, branch enviado (`git push -u origin worktree-agent-t2`). Não espere o CI.
Termine dizendo: commits, o que rodou localmente, o que não foi testado de verdade.
```

### T4: acabamento mecânico (Sonnet)

```
Worktree: C:\FuseDev\fusemovie\.claude\worktrees\t4 (branch worktree-agent-t4). Trabalhe só nela;
todo comando começa com `cd /c/FuseDev/fusemovie/.claude/worktrees/t4 &&`.
CARGO_TARGET_DIR=C:\FuseDev\target-t4. As regras estão no topo do AGENTS.md (seção Fusemovie).

1. crates/ui-egui/src/studio/home.rs e new_doc.rs: há uma paleta privada marcada
   `// integrator: switch to theme::Tokens` (2 marcas). Troque por `app.tokens` (veja como
   studio/top_bar.rs usa) e apague a paleta. Não mude o desenho. Não mexa na função do trilho
   (item "Criar com IA"): outro agente está nela.
2. `grep -rn '"[^"]*FilmCraft[^"]*"' --include=*.rs crates/ui-egui/src apps/filmcraft/src`: nos
   textos que o usuário vê, troque por `{app}`/`crate::brand::APP_NAME` e ajuste a linha do
   catálogo; deixe os de teste, de log e de formato de arquivo. Idem a linha com FilmCraft em
   crates/ui-egui/src/i18n/pt-br.tsv.
3. README.md: substitua por um README curto do Fusemovie (o que é, que é um fork de
   storytold/filmcraft sob MIT OR Apache-2.0, onde baixar o instalador, como compilar, link para
   docs/fusemovie/). Guarde o original como docs/upstream-README.md.
4. docs/fusemovie/PLANO.md: marque as fases 2 a 5 como feitas e aponte para ORQUESTRACAO.md.

Pronto quando: clippy de filmcraft-ui-egui com -D warnings passa, tudo em commit, branch enviado
(`git push -u origin worktree-agent-t4`). Não espere o CI. Relatório de 5 linhas.
```

### T3: o que a suíte inteira acusar (Sonnet, só se precisar)

Quando o `full-tests` terminar, baixe os logs (`gh run download <id> -n full-test-logs -R ...`) e
separe as falhas com `grep -E "^test .* FAILED|^error|panicked" *.log`.

- Falhas de glifo CJK ou de fonte ausente não são nossas (o CI não tem `CRAFT_FONTS_DIR`; o
  `04-licoes.md` conta cerca de 18). Ignore.
- `cargo xtask assets` reclamando de PNG sem `.attribution`: provavelmente os de
  `docs/redesign/fusemovie/` e os ícones novos.
- O resto vai para o agente, com a lista já filtrada colada no prompt:

```
Worktree: C:\FuseDev\fusemovie\.claude\worktrees\t3 (branch worktree-agent-t3). Trabalhe só nela;
todo comando começa com `cd /c/FuseDev/fusemovie/.claude/worktrees/t3 &&`.
CARGO_TARGET_DIR=C:\FuseDev\target-t3. As regras estão no topo do AGENTS.md (seção Fusemovie).

A suíte inteira rodou pela primeira vez no fork. Estas falhas são nossas (lista abaixo). As causas
prováveis: o layout padrão virou o Studio (testes do layout antigo precisam de
`FilmcraftApp::new_classic`), a marca virou `{app}`/Fusemovie (testes que esperam "FilmCraft" ou os
itens de Ajuda e atualização removidos), e assets novos sem sidecar `.attribution` + linha no
ATTRIBUTION.md (formato no AGENTS.md §1.3).
Para cada falha: abra o teste, confirme a causa, corrija o teste; só mude código de produção se o
teste provar um defeito real, e então diga qual. Não apague teste.

<LISTA DE FALHAS FILTRADA>

Pronto quando: clippy --all-targets -D warnings de filmcraft-ui-egui passa, tudo em commit, branch
enviado. Não espere o CI. Termine com: falha → causa → correção, uma linha cada.
```

Depois do merge de T3, dispare o `full-tests` de novo. Se sobrarem até 5 falhas, mande a lista ao
mesmo agente. Mais que isso, pare e mostre ao dono.

## 6. A receita da montagem automática (para T2)

Decisões já tomadas, para o agente não reabrir:

- O material fica no bin do projeto; o Codex olha os clipes com `render_preview` e o parâmetro `item`
  (T1). Não há sequência de referência.
- No máximo 6 quadros por clipe, `max_side` 512: vídeo pesa, e cada imagem custa tokens do dono.
- Legendas saem da transcrição (`transcript.createCaptions`), não de texto digitado pelo modelo.
- Títulos são texto editável (`graphics.newText`). Cor é ajuste por clipe no Lumetri, nunca renderizada.
- O app, não o modelo, cria o projeto, importa e baixa o modelo de fala.

Texto (em inglês, como no Photo; `{language}` é o idioma da interface, `{sequence}` o nome traduzido):

```
Assemble a first cut from the media in this project, following the briefing below. The project is
already open: every clip is imported in the Project bin and the empty sequence "{sequence}" is active.
Work only through this app's tools. Write to the user in {language}.

Progress: begin each message with the tag of the stage you are starting, once per stage and in order:
[[1]] inventory, [[2]] transcription, [[3]] assembly, [[4]] captions and titles, [[5]] music and color,
[[6]] checking. Keep messages to a sentence or two.

1. Inventory. Read the clips with `project_inspect` (ids, durations, sizes). Look at each clip with
   `render_preview` and its `item` parameter at a few evenly spaced times: at most 6 frames per clip,
   `max_side` 512. Note what each clip shows, which are takes of the same moment, and which parts are
   unusable (out of focus, camera moving to position, slate).

2. Transcription. Run `transcript.generate` on the clips with speech, then read the words and their
   times. Speech decides where a cut can fall: never cut inside a word.

3. Assembly. Build the cut in the order the briefing asks, with `command_batch`: place each chosen
   range on the timeline (`timeline.place`), trim heads and tails (`timeline.trim`, `timeline.razor`),
   keep dialogue on the first audio track. Prefer straight cuts; add a transition
   (`sequence.applyVideoTransition`) only where time or place changes. Leave no gaps. If the briefing
   gives a duration, meet it within 5%.

4. Captions and titles. Create captions from the transcript (`transcript.createCaptions`), two lines
   at most. Add the titles the briefing asks for as editable text (`graphics.newText`): opening title,
   names, closing card. Never burn text into an image.

5. Music and color. If a music file is in the bin, put it on its own audio track under the dialogue
   and lower it while people speak (`clip.audioGain`). Make the shots match: adjust exposure, white
   balance and contrast per clip with `lumetri.setSection`, or `lumetri.applyMatch` against the best
   shot of the scene. Adjustments only; do not render or export.

6. Check. Read `sequence_inspect` for gaps, overlaps and clips cut inside a word. Look at
   `render_preview` of the sequence at every cut and at each title. Fix what you find.

Use `command_list` to confirm a command's parameters before the first time you use it. Don't save,
export or close the project: the user saves it when you finish. End with a brief note of the
structure you built, what you checked and what is still rough.

Briefing:
{briefing}
```

Continuação (outra conta ou depois de um limite), antes do mesmo texto:

```
This job was interrupted, and you don't have the earlier conversation. The active sequence already
holds part of the work: inspect it with `sequence_inspect`, `project_inspect` and `render_preview`,
work out which stages are done, and continue from there without redoing them.
```

Sem transcrição no build: tire a etapa 2 e as legendas da etapa 4, e renumere as marcas para 5 etapas.

## 7. Integração de cada onda

1. Para cada branch, na ordem T4, T1, T2, T3: `git merge --no-edit worktree-agent-tN`. Com o
   `merge=union`, os catálogos não devem mais conflitar; se sobrar conflito em código, resolva lendo
   só o trecho em conflito.
2. `git diff --stat ORIG_HEAD HEAD` para ver se alguém saiu dos arquivos da tarefa.
3. Clippy local do conjunto (2 a 3 minutos com cache):
   `CARGO_TARGET_DIR=C:/FuseDev/target-codex cargo clippy -p filmcraft-ui-egui -p filmcraft-automation -p filmcraft -p filmcraft-cli --all-targets -- -D warnings`
4. Um push. Espere com o laço da seção 9. Confirme a pré-release com
   `gh release view latest-studio-redesign -R fernandinhomartins40/fusemovie --json targetCommitish,publishedAt`.
5. Remova as worktrees e os `target-tN`.

## 8. Portão do dono, e a Onda 2

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

## 9. Comandos do orquestrador

Criar as worktrees da Onda 1 (o branch precisa começar com `worktree-agent-` para o `checks.yml` rodar):

```sh
cd /c/FuseDev/fusemovie
for t in t1 t2 t4; do git worktree add -b worktree-agent-$t .claude/worktrees/$t studio-redesign; done
```

Lançar um agente: ferramenta Agent, **sem** `isolation`, com `run_in_background: true`, `model`
(`sonnet` ou `opus`) e o prompt da tarefa.

Esperar o CI de um commit sem gastar tokens (Bash com `run_in_background: true`; avisa quando acabar):

```sh
SHA=<sha>; sleep 60
until out=$(gh run list -R fernandinhomartins40/fusemovie --commit $SHA --json name,status,conclusion,databaseId 2>/dev/null); \
  echo "$out" | grep -q Checks && ! echo "$out" | grep -q '"status":"\(queued\|in_progress\|pending\|waiting\)"'; do sleep 300; done
echo "$out"
```

Trecho do log de uma falha, para mandar ao agente:

```sh
gh run view <id> --log-failed -R fernandinhomartins40/fusemovie | grep -E "error|FAILED|panicked|-->" | head -40
```

Agente caiu no limite de uso: salvar o trabalho e retomar o mesmo agente.

```sh
cd /c/FuseDev/fusemovie/.claude/worktrees
for d in t*; do [ -n "$(git -C $d status --porcelain)" ] && git -C $d add -A && git -C $d commit -q -m "WIP: save work before usage-limit resume"; done
```

Depois: SendMessage ao agente com "retome de onde parou; seu trabalho foi salvo em um commit WIP".

## 10. Fora deste plano

Ficam para depois, por decisão de escopo: assinatura de código do instalador (custa dinheiro; decisão
do dono), abrir `.prproj`, vozes neurais (`neural-voices`), datas na lista de conversas do Codex, coluna
do Codex redimensionável no layout clássico, builds para macOS e Linux.
