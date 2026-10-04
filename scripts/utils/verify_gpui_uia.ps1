param(
    [Parameter(Mandatory)][int]$ProcessId,
    [Parameter(Mandatory)][long]$WindowId,
    [string]$ExpectedTarget,
    [string]$ExpectedSource,
    [int]$ExpectedSegments = -1,
    [switch]$Locked,
    [Parameter(Mandatory)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
@{ session = 'lumencat-verification' } | ConvertTo-Json -Compress | & cua-driver call start_session | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'No se pudo iniciar la sesión de verificación UIA' }
$Request = @{ pid = $ProcessId; window_id = $WindowId; include_screenshot = $false; session = 'lumencat-verification' }
$Json = $Request | ConvertTo-Json -Compress
$StateJson = $Json | & cua-driver call get_window_state
if ($LASTEXITCODE -ne 0) { throw 'No se pudo consultar UIA mediante cua-driver' }
$State = ($StateJson -join "`n") | ConvertFrom-Json
if ($State.truncated -or $State.returned_element_count -ne $State.total_element_count) {
    throw 'Snapshot UIA truncado o filtrado'
}
function Find-Control([string]$Label, [string]$Role) {
    $Items = @($State.elements | Where-Object { $_.label -ceq $Label -and $_.role -eq $Role })
    if ($Items.Count -ne 1) { throw "Se esperaba un control UIA '$Label' ($Role), hay $($Items.Count)" }
    return $Items[0]
}
$Target = Find-Control 'Destino' 'Edit'
$Source = Find-Control 'Origen del segmento activo' 'Edit'
$Status = @($State.elements | Where-Object role -eq 'StatusBar')[0]
if ($PSBoundParameters.ContainsKey('ExpectedTarget') -and [string]$Target.value -cne $ExpectedTarget) {
    throw 'El destino visible por UIA no coincide con el resultado esperado'
}
if ($PSBoundParameters.ContainsKey('ExpectedSource') -and [string]$Source.value -cne $ExpectedSource) {
    throw 'El origen visible por UIA no coincide con el segmento esperado'
}
if ($Locked -and @($State.elements | Where-Object { $_.label -eq 'Confirmar y avanzar' -and $_.enabled }).Count) {
    throw 'Confirmar sigue habilitado con el segmento bloqueado'
}
$Rows = @($State.elements | Where-Object { $_.role -eq 'DataItem' -and $_.label -match '^Segmento \d+$' })
if ($ExpectedSegments -ge 0 -and $Rows.Count -ne $ExpectedSegments) {
    throw "Se esperaban $ExpectedSegments segmentos accesibles, hay $($Rows.Count)"
}
if ($null -eq $Status -or $Status.label -like 'Error*') { throw 'La app no comunica un estado válido' }
$Report = [ordered]@{
    Result = 'PASS'; ProcessId = $ProcessId; WindowId = $WindowId
    Controls = $State.total_element_count; Segments = $Rows.Count
    Status = $Status.label; TargetLength = ([string]$Target.value).Length
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$StateJson | Set-Content -LiteralPath (Join-Path $OutputDirectory 'uia-state.json') -Encoding utf8
$Report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $OutputDirectory 'uia-report.json') -Encoding utf8
[pscustomobject]$Report
