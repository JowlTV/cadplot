# Gera o relatório de importação de todos os DWG/DXF de uma pasta (padrão: samples\).
# Os relatórios ficam ao lado de cada desenho (*.report.txt e *.report.json),
# que o .gitignore mantém fora do repositório. Os desenhos não são alterados.
#
# Uso (PowerShell, na raiz do repositório):
#   .\scripts\relatorio-corpus.ps1                 # usa samples\
#   .\scripts\relatorio-corpus.ps1 -Pasta D:\obras -Fontes "C:\Fontes SHX"
param(
    [string]$Pasta = "samples",
    [string]$Fontes = "",
    [string]$Exe = ""
)
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if ($Exe -eq "") {
    $Exe = Join-Path $PSScriptRoot "..\target\release\cadplot-inspect.exe"
    if (-not (Test-Path $Exe)) { $Exe = Join-Path $PSScriptRoot "cadplot-inspect.exe" }
}
if (-not (Test-Path $Exe)) {
    Write-Error "cadplot-inspect.exe não encontrado. Rode 'cargo build --release -p cadplot-inspect' ou passe -Exe."
}

$arquivos = Get-ChildItem -Path $Pasta -Recurse -File -Include *.dwg, *.dxf
if ($arquivos.Count -eq 0) { Write-Host "Nenhum DWG/DXF em $Pasta"; exit 0 }

$resumo = @()
foreach ($a in $arquivos) {
    $args = @($a.FullName, "--json-ao-lado", "--txt-ao-lado", "--linhas", "0")
    if ($Fontes -ne "") { $args += @("--fontes", $Fontes) }
    $inicio = Get-Date
    & $Exe @args | Out-Null
    $codigo = $LASTEXITCODE
    $ms = [int]((Get-Date) - $inicio).TotalMilliseconds
    $json = "$($a.FullName).report.json"
    if (Test-Path $json) {
        $r = Get-Content $json -Raw -Encoding UTF8 | ConvertFrom-Json
        $resumo += [pscustomobject]@{
            Arquivo   = $a.Name
            Versao    = $r.file.version_code
            Entidades = $r.entities.total
            Camadas   = $r.layers.Count
            Layouts   = $r.layouts.Count
            Avisos    = ($r.warnings | Where-Object { $_.level -ne "info" }).Count
            TempoMs   = $ms
            Codigo    = $codigo
        }
    } else {
        $resumo += [pscustomobject]@{ Arquivo = $a.Name; Versao = "ERRO"; Entidades = 0; Camadas = 0; Layouts = 0; Avisos = 0; TempoMs = $ms; Codigo = $codigo }
    }
}
$resumo | Format-Table -AutoSize
