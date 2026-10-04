param(
    [Parameter(Mandatory)][ValidateSet('launch', 'reopen', 'doctor', 'snapshot', 'click', 'type', 'key', 'hotkey', 'cleanup')][string]$Action,
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$')][string]$RunId,
    [ValidateSet('gpui', 'legacy')][string]$Surface = 'gpui',
    [long]$WindowId = 0,
    [double]$X = -1,
    [double]$Y = -1,
    [string]$Label,
    [string]$Role,
    [string]$Text,
    [string[]]$Keys,
    [int]$WaitSeconds = 0,
    [switch]$ForceStop
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$Root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$Directory = Join-Path $Root "output/verification/$RunId"
$ManifestPath = Join-Path $Directory 'instance.json'
$LockPath = Join-Path $Root 'output/verification/.app-lock.json'

function Read-Lock {
    if (!(Test-Path -LiteralPath $LockPath)) { return $null }
    try { return Get-Content -LiteralPath $LockPath -Raw | ConvertFrom-Json } catch { return $null }
}

function Write-Lock([string]$OwnerRunId) {
    @{ runId = $OwnerRunId; heartbeatUtc = [DateTime]::UtcNow.ToString('o') } |
        ConvertTo-Json -Compress | Set-Content -LiteralPath $LockPath -Encoding utf8
}

function Request-Lock([string]$OwnerRunId, [int]$WaitSeconds) {
    $Owner = $null
    $Deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    do {
        $Owner = Read-Lock
        $Stale = !$Owner -or ([DateTime]::UtcNow - ([datetime]$Owner.heartbeatUtc)).TotalMinutes -gt 10
        if (!$Owner -or $Stale -or $Owner.runId -eq $OwnerRunId) {
            Write-Lock $OwnerRunId
            return
        }
        Start-Sleep -Seconds 2
    } while ([DateTime]::UtcNow -lt $Deadline)
    throw "La app está en cola para la ejecución $($Owner.runId); espera con -WaitSeconds o ejecuta su cleanup. Cola: $LockPath"
}

function Release-Lock([string]$OwnerRunId) {
    $Owner = Read-Lock
    if ($Owner -and $Owner.runId -eq $OwnerRunId) { Remove-Item -LiteralPath $LockPath -Force }
}

function Invoke-Driver([string]$Tool, [hashtable]$Arguments, [string]$Evidence) {
    $Raw = & cua-driver $Tool ($Arguments | ConvertTo-Json -Depth 8 -Compress)
    $Code = $LASTEXITCODE
    if ($Evidence) { $Raw | Set-Content -LiteralPath $Evidence -Encoding utf8 }
    if ($Code -ne 0) { throw "cua-driver $Tool falló ($Code); evidencia: $Evidence" }
    $Result = ($Raw -join "`n") | ConvertFrom-Json
    if ($Result.isError) { throw "cua-driver $Tool rechazó la acción; evidencia: $Evidence" }
    if ($Result.structuredContent) { return $Result.structuredContent }
    return $Result
}

function Save-Manifest($Manifest) {
    $Manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ManifestPath -Encoding utf8
}

function Get-OwnedProcess($Manifest) {
    $Process = Get-Process -Id $Manifest.pid -ErrorAction SilentlyContinue
    if (!$Process) { return $null }
    $Executable = $Process.Path
    if (!$Executable) {
        $Info = Invoke-Driver 'debug_window_info' @{ pid = $Manifest.pid } ''
        $Executable = $Info.exe_path
    }
    if ($Executable -ne $Manifest.executable -or $Process.StartTime.ToUniversalTime() -ne ([datetime]$Manifest.startedUtc).ToUniversalTime()) {
        throw 'El PID ya no pertenece a esta ejecución; no se actuará sobre él.'
    }
    return $Process
}

function Get-Doctor($Manifest) {
    $Process = Get-OwnedProcess $Manifest
    if (!$Process) { throw 'La instancia registrada ya no está en ejecución.' }
    if ((Get-FileHash -LiteralPath $Manifest.executable -Algorithm SHA256).Hash -ne $Manifest.sha256) {
        throw 'El ejecutable cambió desde launch; cerrar esta instancia antes de reconstruir.'
    }
    $Details = Get-CimInstance Win32_Process -Filter "ProcessId = $($Manifest.pid)"
    if ($Details.CommandLine -and !$Details.CommandLine.Contains($Manifest.project)) { throw 'La línea de comandos no identifica el proyecto aislado.' }
    $Windows = Invoke-Driver 'list_windows' @{ pid = $Manifest.pid } ''
    $Selected = if ($WindowId) { $WindowId } else { $Manifest.windowId }
    if (!@($Windows.windows | Where-Object window_id -eq $Selected).Count) {
        throw 'La ventana registrada no pertenece a esta instancia; revisar list_windows.'
    }
    return [pscustomobject]@{
        pid = $Manifest.pid; windowId = $Selected; project = $Manifest.project
        executable = $Manifest.executable; sha256 = $Manifest.sha256
        surface = $Manifest.surface; evidence = $Directory; windows = $Windows.windows
        commandLineVerified = [bool]$Details.CommandLine
    }
}

function Save-Snapshot($Doctor, [string]$Prefix) {
    $Arguments = @{
        pid = $Doctor.pid; window_id = $Doctor.windowId
        screenshot_out_file = (Join-Path $Directory "$Prefix.png")
    }
    if ($Label) { $Arguments.query = $Label }
    return Invoke-Driver 'get_window_state' $Arguments (Join-Path $Directory "$Prefix.json")
}

if ($Action -eq 'launch') {
    Request-Lock $RunId $WaitSeconds
    try {
        if (Test-Path -LiteralPath $Directory) { throw 'RunId existente: usa otro o reopen; nunca se sobrescribe evidencia.' }
        $Build = Join-Path $Root '.cache/target/debug/lumencat.exe'
        if (!(Test-Path -LiteralPath $Build)) { throw 'Ejecuta cargo build --locked --bin lumencat primero.' }
        New-Item -ItemType Directory -Path $Directory | Out-Null
        $Executable = Join-Path $Directory 'lumencat.exe'
        Copy-Item -LiteralPath $Build -Destination $Executable
        [IO.File]::WriteAllText((Join-Path $Directory 'source.txt'), "Privacy notice 42.`r`nHello world`r`nPrivacy notice 42.`r`nUnicode café 世界`r`nVisit https://example.test/{name}.`r`n")
        [IO.File]::WriteAllText((Join-Path $Directory 'sample.xlf'), '<?xml version="1.0" encoding="utf-8"?><xliff version="1.2"><file original="sample" source-language="en" target-language="es" datatype="plaintext"><body><trans-unit id="1"><source>XLIFF example.</source><target>Ejemplo XLIFF.</target></trans-unit></body></file></xliff>')
        [IO.File]::WriteAllText((Join-Path $Directory 'sample.tmx'), '<?xml version="1.0" encoding="utf-8"?><tmx version="1.4"><header creationtool="Verification" creationtoolversion="1" segtype="sentence" o-tmf="LumenCAT" adminlang="en" srclang="en" datatype="PlainText"/><body><tu><tuv xml:lang="en"><seg>Hello world</seg></tuv><tuv xml:lang="es"><seg>Hola mundo</seg></tuv></tu></body></tmx>')
        $Manifest = [pscustomobject]@{
            runId = $RunId; surface = $Surface; executable = $Executable
            sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash
            project = (Join-Path $Directory 'project.lcat'); pid = 0; windowId = 0
            startedUtc = ''; closedUtc = $null
        }
    } catch {
        Release-Lock $RunId
        throw
    }
} else {
    if (!(Test-Path -LiteralPath $ManifestPath)) { throw "No existe instancia registrada: $ManifestPath" }
    $Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    Request-Lock $RunId $WaitSeconds
}

if ($Action -in @('launch', 'reopen')) {
    if ($Action -eq 'reopen') {
        if (Get-OwnedProcess $Manifest) { throw 'La instancia anterior sigue abierta; usa cleanup primero.' }
        if (!(Test-Path -LiteralPath $Manifest.project)) { throw 'No existe proyecto para reabrir.' }
        if ((Get-FileHash -LiteralPath $Manifest.executable -Algorithm SHA256).Hash -ne $Manifest.sha256) { throw 'La build cambió; crea una ejecución nueva.' }
    }
    $Arguments = @('--project', $Manifest.project)
    if ($Manifest.surface -eq 'legacy') { $Arguments += '--legacy-egui' }
    $Stamp = Get-Date -Format 'yyyyMMdd-HHmmss-ffff'
    $Launch = Invoke-Driver 'launch_app' @{
        path = $Manifest.executable; additional_arguments = $Arguments
        creates_new_application_instance = $true
    } (Join-Path $Directory "$Stamp-launch.json")
    $Manifest.pid = $Launch.pid
    $Process = Get-Process -Id $Manifest.pid
    $Manifest.startedUtc = $Process.StartTime.ToUniversalTime().ToString('o')
    $Manifest.closedUtc = $null
    Save-Manifest $Manifest
    $Deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        $Windows = Invoke-Driver 'list_windows' @{ pid = $Manifest.pid } ''
        $Main = @($Windows.windows | Where-Object { $_.title -like '*LumenCAT*' })
        if ($Main.Count -eq 1) { break }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $Deadline)
    if ($Main.Count -ne 1) { throw "Ventana no identificable; cleanup de RunId $RunId antes de reintentar." }
    $Manifest.windowId = $Main[0].window_id
    Save-Manifest $Manifest
    $Doctor = Get-Doctor $Manifest
    Save-Snapshot $Doctor "$Stamp-ready" | Out-Null
    Write-Lock $RunId
    $Doctor | ConvertTo-Json -Depth 8
    exit 0
}

if ($Action -eq 'cleanup') {
    $Process = Get-OwnedProcess $Manifest
    if ($Process) {
        $Doctor = Get-Doctor $Manifest
        $Stamp = Get-Date -Format 'yyyyMMdd-HHmmss-ffff'
        $Snapshot = Save-Snapshot $Doctor "$Stamp-before-close"
        $Close = @($Snapshot.elements | Where-Object { $_.role -eq 'button' -and $_.label -match '^(Close|Cerrar)$' })
        if ($Close.Count -eq 1) {
            Invoke-Driver 'click' @{
                pid = $Doctor.pid; window_id = $Doctor.windowId
                element_token = $Close[0].element_token; delivery_mode = 'background'
            } (Join-Path $Directory "$Stamp-close.json") | Out-Null
            $Process.WaitForExit(10000) | Out-Null
        }
        if (Get-OwnedProcess $Manifest) {
            if (!$ForceStop) { throw 'Cierre no terminado; conserva borrador/evidencia. ForceStop solo para esta instancia desechable.' }
            Invoke-Driver 'kill_app' @{ pid = $Manifest.pid } (Join-Path $Directory "$Stamp-force-stop.json") | Out-Null
            $Process.WaitForExit(5000) | Out-Null
            if (Get-OwnedProcess $Manifest) { throw 'La instancia no terminó.' }
        }
    }
    $Manifest.closedUtc = [DateTime]::UtcNow.ToString('o')
    Save-Manifest $Manifest
    Release-Lock $RunId
    [pscustomobject]@{ stopped = $true; evidencePreserved = (Test-Path -LiteralPath $ManifestPath); evidence = $Directory } | ConvertTo-Json
    exit 0
}

$Doctor = Get-Doctor $Manifest
if ($Action -eq 'doctor') { Write-Lock $RunId; $Doctor | ConvertTo-Json -Depth 8; exit 0 }
$Stamp = Get-Date -Format 'yyyyMMdd-HHmmss-ffff'
$Before = Save-Snapshot $Doctor "$Stamp-before-$Action"
if ($Action -eq 'snapshot') {
    $Before | Select-Object pid, window_id, window_title, snapshot_id, screenshot_file_path, screenshot_width, screenshot_height, elements | ConvertTo-Json -Depth 8
    exit 0
}
$Arguments = @{ pid = $Doctor.pid; window_id = $Doctor.windowId; delivery_mode = 'background' }
if ($Label) {
    $Elements = @($Before.elements | Where-Object { $_.label -eq $Label -and (!$Role -or $_.role -eq $Role) })
    if ($Elements.Count -ne 1) { throw 'Label/Role no identifica un único elemento de la captura actual.' }
    $Arguments.element_token = $Elements[0].element_token
} elseif ($Action -in @('click', 'type')) {
    if ($X -lt 0 -or $Y -lt 0 -or $X -ge $Before.screenshot_width -or $Y -ge $Before.screenshot_height) { throw 'Coordenadas fuera de la captura actual.' }
    $Arguments.x = $X; $Arguments.y = $Y
}
$Tool = switch ($Action) {
    'click' { 'click' }
    'type' { $Arguments.text = $Text; 'type_text' }
    'key' { if ($Keys.Count -ne 1) { throw 'key requiere una tecla.' }; $Arguments.key = $Keys[0]; 'press_key' }
    'hotkey' { if (!$Keys.Count) { throw 'hotkey requiere Keys.' }; $Arguments.keys = $Keys; 'hotkey' }
}
try {
    $Result = Invoke-Driver $Tool $Arguments (Join-Path $Directory "$Stamp-action-$Action.json")
    $Result | ConvertTo-Json -Depth 16
} finally {
    $Windows = Invoke-Driver 'list_windows' @{ pid = $Manifest.pid } ''
    if (!@($Windows.windows | Where-Object window_id -eq $Doctor.windowId).Count) { $Doctor.windowId = $Manifest.windowId }
    Save-Snapshot $Doctor "$Stamp-after-$Action" | Out-Null
    Write-Lock $RunId
}
