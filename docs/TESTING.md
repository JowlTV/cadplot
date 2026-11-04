# Como testar no Windows

Instruções por milestone. A mais recente fica no topo.

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

Com a tag `v0.0.0`, o mesmo `.exe` também fica na página **Releases**.

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
