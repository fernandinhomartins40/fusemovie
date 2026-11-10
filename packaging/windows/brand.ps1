# Reads brand.env (repository root), the one place the product name lives, into a hashtable:
# NAME, COMPANY, ID, FILE. Dot-source it: `. (Join-Path $PSScriptRoot 'brand.ps1')` gives $Brand.
$Brand = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $PSScriptRoot '..\..\brand.env')) {
  if ($line -match '^\s*BRAND_(\w+)\s*=\s*(.*?)\s*$') { $Brand[$Matches[1]] = $Matches[2] }
}
foreach ($key in 'NAME', 'COMPANY', 'FILE') {
  if (-not $Brand[$key]) { throw "brand.env has no BRAND_$key" }
}
