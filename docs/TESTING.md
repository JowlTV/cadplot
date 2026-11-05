# Como testar no Windows

Instruções por milestone. A mais recente fica no topo.

---

## Prévia do M1 — app CadPlot (visualizador + camadas)

Primeira versão com interface. Abre DWG/DXF (somente leitura), mostra o **espaço modelo** com pan e
zoom, e tem o painel de camadas. Ainda é prévia: o M1 continua até fechar o que está em
"Limitações" abaixo.

### 1. Obter

Em <https://github.com/JowlTV/cadplot/actions>, abra a execução mais recente do **CI** no branch
`feat/m1-viewer` e baixe o artefato **cadplot-windows-x64**. Dentro:

- `cadplot.exe` — o app (um arquivo só, sem instalador);
- `cadplot-inspect.exe` — a CLI do relatório (M0);
- `SHA256SUMS.txt`.

O app usa o **WebView2**, que já vem no Windows 11 e na maioria dos Windows 10 atualizados. Se ele
abrir em branco ou reclamar do WebView2, instale o "Evergreen Runtime" da Microsoft (o instalador do
M5 fará isso sozinho). O `.exe` ainda **não é assinado**: o Windows SmartScreen pode avisar
("Mais informações → Executar assim mesmo"). Se o Bitdefender bloquear, anote o nome da detecção.

Compilar você mesmo: instale Rust e Node 22, depois

```powershell
cd apps\desktop
npm ci
npx tauri build --no-bundle      # gera target\release\cadplot.exe
```

### 2. Usar

- **Abrir:** botão *Abrir…* (Ctrl+O), arrastar o DWG para a janela, ou "Abrir com → cadplot.exe".
- **Navegar:** roda do mouse = zoom no cursor; arrastar com botão esquerdo ou do meio = pan;
  duplo clique = zoom extensão; *Zoom janela* ou **Shift + arrastar** = zoom por janela; Esc cancela.
- **Camadas:** ● liga/desliga, ❄ congela (congelar a camada de um INSERT esconde o bloco inteiro, como
  no AutoCAD), *Isolar* deixa só aquela camada (passe o mouse na linha), busca por nome, agrupamento
  por prefixo com mostrar/ocultar/isolar do grupo inteiro. Camadas em itálico não têm nada no espaço
  modelo (em geral estão só no layout ou no carimbo).
- **Relatório:** mostra versão, avisos e a tabela "o que esta versão desenha", com o que foi
  desenhado, aproximado ou não desenhado e o motivo (passe o mouse na linha).
- **Fundo** claro/escuro; **Textos** liga/desliga todos os textos.
- Nada é gravado: nem o DWG, nem arquivo de estado (o estado das camadas some ao fechar; o
  `.cadplot.json` é o M3).

### 3. O que observar

Abra os DWG reais (`0004-DETA`, `0002-TERR` e outros) e compare com o AutoCAD:

| Ponto | Esperado |
|---|---|
| Abre sem erro | sim, em menos de 1 s para arquivos comuns |
| Geometria | linhas, arcos, polilinhas com arco, círculos, elipses, splines, blocos (com rotação/escala/espelho), cotas, hachuras sólidas no lugar certo |
| Cores | iguais às do AutoCAD (PorCamada, PorBloco, ACI, RGB); branco vira preto no fundo claro |
| Camadas | ligar/desligar/congelar/isolar se comportam como no AutoCAD, inclusive em blocos |
| Textos e acentos | aparecem na posição e rotação certas, acentos corretos (fonte ainda substituta) |
| Coordenadas | X/Y no rodapé batem com o AutoCAD no mesmo ponto |
| Desempenho | pan e zoom fluidos; diga o tamanho do maior arquivo testado |
| DPI | teste em 100 %, 150 % e 200 % de escala do Windows: linhas nítidas, interface legível |

Mande prints de qualquer diferença (CadPlot × AutoCAD lado a lado ajuda muito).

### Limitações conhecidas desta prévia

- Só o espaço modelo; layouts/viewports entram no M1 completo (com a escala efetiva já calculada no
  relatório).
- Tipos de linha aparecem contínuos; espessura de linha com 1 pixel.
- Hachuras com padrão aparecem só com o contorno (sólidas são preenchidas).
- Textos em fonte substituta (Arial); SHX ainda não são lidas; MTEXT sem quebra automática de linha.
- Setas de LEADER/MLEADER, WIPEOUT, imagens, MLINE e 3D não são desenhados (listados no relatório).
- Sem seleção/inspetor (M2), regras de limpeza (M3) nem plotagem (M4).

---

## M0 — spike de leitura (`cadplot-inspect`)

O M0 ainda não tem interface gráfica: entrega um programa de linha de comando que abre um DWG/DXF
**somente para leitura** e imprime o relatório de importação. O objetivo é confirmar que a leitura dos
seus arquivos reais funciona antes de construir o visualizador.

### 1. Obter o executável

**Opção A — baixar pronto (recomendado).** Em <https://github.com/JowlTV/cadplot/actions>, abra a
execução mais recente do workflow **CI** no branch `feat/m0-spike` (ou `main`, depois do merge) e baixe o
artefato **cadplot-inspect-windows-x64**. O zip contém:

- `cadplot-inspect.exe`
- `SHA256SUMS.txt` (confira com `Get-FileHash .\cadplot-inspect.exe`)
- `fixtures-sinteticos\` (desenhos gerados por código, para comparar)
- este `TESTING.md`

Depois do merge do M0 e da tag `v0.0.0`, o mesmo `.exe` também fica na página **Releases**.

**Opção B — compilar.**

1. Instale o Rust: <https://rustup.rs> (escolha o padrão `x86_64-pc-windows-msvc`; o instalador oferece
   as *Visual Studio Build Tools* se faltarem).
2. No PowerShell:
   ```powershell
   git clone https://github.com/JowlTV/cadplot
   cd cadplot
   git checkout feat/m0-spike   # ou main, depois do merge
   cargo test --workspace
   cargo build --release -p cadplot-inspect
   ```
3. O executável fica em `target\release\cadplot-inspect.exe`.

O executável não é empacotado nem ofuscado e ainda **não é assinado** (assinatura no M5). Se o
Bitdefender bloquear, anote a mensagem exata e o nome da detecção.

### 2. Rodar no arquivo real

Copie o DWG para `samples\` (pasta ignorada pelo git, nada dali sobe para o GitHub) e rode:

```powershell
.\target\release\cadplot-inspect.exe "samples\1361-PSG-AP-0002-TERR-R01.dwg" --txt-ao-lado --json-ao-lado
```

Se a empresa tem uma pasta de fontes SHX, aponte-a para o relatório dizer o que falta de verdade:

```powershell
.\target\release\cadplot-inspect.exe "samples\1361-PSG-AP-0002-TERR-R01.dwg" --fontes "C:\caminho\das\fontes" --txt-ao-lado
```

Para vários arquivos de uma vez (gera um resumo em tabela):

```powershell
.\scripts\relatorio-corpus.ps1                    # tudo em samples\
.\scripts\relatorio-corpus.ps1 -Fontes "C:\Fontes SHX"
```

Os relatórios ficam ao lado do desenho: `…dwg.report.txt` e `…dwg.report.json`. O DWG não é alterado;
para conferir, compare o SHA-256 do arquivo antes e depois (`Get-FileHash`), que deve ser igual ao
impresso no relatório.

### 3. O que observar (e me mandar)

Envie o `.report.txt` (e, se possível, o `.report.json`) do arquivo `1361-PSG-AP-0002-TERR-R01.dwg`.
Os relatórios contêm nomes de camadas e textos do projeto, mas não a geometria. Confira contra o AutoCAD:

| Item | Onde ver no relatório | Comparar com o AutoCAD |
|---|---|---|
| Versão | `Versão: AC1032` | o arquivo é do 2026, deve ser AC1032 |
| Leitura | `Leitura: estrita, ok` | se aparecer **MODO DE RECUPERAÇÃO**, é o primeiro problema a investigar |
| Acentos | seção CODIFICAÇÃO e nomes em CAMADAS | `PA-COMENTÁRIO` e outros nomes devem aparecer idênticos; `com caractere inválido (�): 0` |
| Camadas | CAMADAS | quantidade e estados (ligada/congelada/travada) iguais ao gerenciador de camadas (`LAYER`) |
| Contagens | ENTIDADES | algumas contagens com a `QSELECT` ou `BCOUNT` do AutoCAD |
| Layouts | LAYOUTS | nomes das abas, tamanho do papel e escala de cada viewport (ex.: `1:50`) |
| Fontes | FONTES | SHX marcadas como FALTANDO que você tem instaladas no AutoCAD |
| Xrefs | XREFS | se o desenho tiver referências, se o caminho está certo |
| Tempo | `Tempo:` | quanto levou para abrir |
| Avisos | AVISOS | tipos "não renderizados" e objetos desconhecidos |

Se tiver um DWG salvo no **AutoCAD 2027** e um **DXF** exportado do mesmo projeto, rode também e mande os
relatórios: o M0 fecha a lista de versões testadas com eles.

### 4. Testes automáticos

`cargo test --workspace` roda mais de 20 testes, incluindo importação dos fixtures sintéticos em todas as
versões e a guarda de somente-leitura. O teste de desempenho (500 mil entidades) roda à parte:

```powershell
cargo test --release -p cadplot-core --test import_fixtures -- --ignored desempenho --nocapture
```

Ele imprime o tempo; o critério é abrir em menos de 10 s.
