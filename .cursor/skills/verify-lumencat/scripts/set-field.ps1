param(
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$')][string]$RunId,
    [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$Label,
    [Parameter(Mandatory)][AllowEmptyString()][string]$Value,
    [long]$WindowId = 0,
    [int]$WaitSeconds = 900
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$Root = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path
$Controller = Join-Path $Root 'scripts/utils/control_lumencat.ps1'
$Target = @{ RunId = $RunId; WindowId = $WindowId; WaitSeconds = $WaitSeconds }
$Doctor = & $Controller -Action doctor @Target | ConvertFrom-Json
$Before = & $Controller -Action snapshot @Target -Label $Label -Role Edit | ConvertFrom-Json
$Fields = @($Before.elements | Where-Object { $_.label -eq $Label -and $_.role -eq 'Edit' })
if ($Fields.Count -ne 1) { throw 'Label no identifica un único campo Edit; no se modificó ningún valor.' }
$Stamp = Get-Date -Format 'yyyyMMdd-HHmmss-ffff'
$Evidence = Join-Path $Doctor.evidence "$Stamp-set-value.json"
$Result = $null
try {
    $Arguments = @{
        pid = $Doctor.pid; window_id = $Doctor.windowId
        element_token = $Fields[0].element_token; value = $Value
    }
    $Raw = & cua-driver set_value ($Arguments | ConvertTo-Json -Compress)
    $Code = $LASTEXITCODE
    $Raw | Set-Content -LiteralPath $Evidence -Encoding utf8
    if ($Code -ne 0) { throw "SetValue falló ($Code); evidencia: $Evidence" }
    $Result = ($Raw -join "`n") | ConvertFrom-Json
    if ($Result.isError) { throw "SetValue rechazado; evidencia: $Evidence" }
} finally {
    $After = & $Controller -Action snapshot @Target -Label $Label -Role Edit | ConvertFrom-Json
}
$Observed = @($After.elements | Where-Object { $_.label -eq $Label -and $_.role -eq 'Edit' })
if ($Observed.Count -ne 1 -or [string]$Observed[0].value -cne $Value) {
    throw "El valor leído no coincide con el solicitado; revisa captura y doctor. Evidencia: $Evidence"
}
[pscustomobject]@{
    label = $Label; valueVerified = $true; actionEvidence = $Evidence
    beforeScreenshot = $Before.screenshot_file_path; afterScreenshot = $After.screenshot_file_path
    actionResult = $Result
} | ConvertTo-Json -Depth 8
