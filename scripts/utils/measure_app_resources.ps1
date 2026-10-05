param(
    [Parameter(Mandatory)][int[]]$RootProcessIds,
    [Parameter(Mandatory)][string[]]$Names,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateRange(1, 3600)][int]$DurationSeconds = 30,
    [ValidateRange(100, 5000)][int]$IntervalMilliseconds = 500
)

$ErrorActionPreference = 'Stop'
if ($RootProcessIds.Count -ne $Names.Count) { throw 'Cada PID requiere un nombre.' }
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$logicalProcessors = [Environment]::ProcessorCount
$roots = @{}
for ($i = 0; $i -lt $RootProcessIds.Count; $i++) {
    $process = Get-Process -Id $RootProcessIds[$i]
    $roots[$process.Id] = @{ Name = $Names[$i]; Started = $process.StartTime.ToUniversalTime() }
}
$previous = @{}
$rows = [Collections.Generic.List[object]]::new()
$timer = [Diagnostics.Stopwatch]::StartNew()
while ($timer.Elapsed.TotalSeconds -lt $DurationSeconds) {
    $tree = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, CreationDate)
    foreach ($rootId in $RootProcessIds) {
        $root = Get-Process -Id $rootId -ErrorAction SilentlyContinue
        if (!$root -or $root.StartTime.ToUniversalTime() -ne $roots[$rootId].Started) { throw "El PID $rootId cambió o terminó." }
        $ids = [Collections.Generic.HashSet[int]]::new()
        [void]$ids.Add($rootId)
        do {
            $added = $false
            foreach ($entry in $tree) {
                if ($ids.Contains([int]$entry.ParentProcessId) -and !$ids.Contains([int]$entry.ProcessId)) {
                    $parent = $tree | Where-Object ProcessId -eq $entry.ParentProcessId | Select-Object -First 1
                    if ($parent -and $entry.CreationDate -ge $parent.CreationDate) {
                        [void]$ids.Add([int]$entry.ProcessId)
                        $added = $true
                    }
                }
            }
        } while ($added)
        $workingSet = 0L; $privateBytes = 0L; $cpuDelta = 0.0; $count = 0
        $now = $timer.Elapsed.TotalSeconds
        foreach ($processId in $ids) {
            $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if (!$process) { continue }
            $key = "$processId/$($process.StartTime.ToUniversalTime().Ticks)"
            $cpu = $process.TotalProcessorTime.TotalSeconds
            if ($previous.ContainsKey($key)) { $cpuDelta += [math]::Max(0, $cpu - $previous[$key]) }
            $previous[$key] = $cpu
            $workingSet += $process.WorkingSet64
            $privateBytes += $process.PrivateMemorySize64
            $count++
        }
        $elapsed = if ($previous.ContainsKey("time/$rootId")) { $now - $previous["time/$rootId"] } else { 0 }
        $previous["time/$rootId"] = $now
        $phasePath = Join-Path $OutputDirectory 'phase.txt'
        $phase = if (Test-Path -LiteralPath $phasePath) { (Get-Content -Raw -LiteralPath $phasePath).Trim() } else { 'unspecified' }
        $rows.Add([pscustomobject]@{
            Utc = [DateTime]::UtcNow.ToString('o'); ElapsedSeconds = $now
            App = $roots[$rootId].Name; RootPid = $rootId; Phase = $phase
            ProcessCount = $count; WorkingSetMiB = $workingSet / 1MB; PrivateMiB = $privateBytes / 1MB
            CpuPercentMachine = if ($elapsed -gt 0) { 100 * $cpuDelta / $elapsed / $logicalProcessors } else { $null }
            CpuSecondsDelta = $cpuDelta; LogicalProcessors = $logicalProcessors
        })
    }
    Start-Sleep -Milliseconds $IntervalMilliseconds
}
$rows | Export-Csv -NoTypeInformation -Encoding utf8 -LiteralPath (Join-Path $OutputDirectory 'resources.csv')
$rows | Group-Object App, Phase | ForEach-Object {
    $valid = @($_.Group | Where-Object { $null -ne $_.CpuPercentMachine })
    [pscustomobject]@{
        App = $_.Group[0].App; Phase = $_.Group[0].Phase; Samples = $_.Count
        WorkingSetMeanMiB = ($_.Group.WorkingSetMiB | Measure-Object -Average).Average
        WorkingSetPeakMiB = ($_.Group.WorkingSetMiB | Measure-Object -Maximum).Maximum
        PrivateMeanMiB = ($_.Group.PrivateMiB | Measure-Object -Average).Average
        CpuMeanPercentMachine = ($valid.CpuPercentMachine | Measure-Object -Average).Average
        CpuPeakPercentMachine = ($valid.CpuPercentMachine | Measure-Object -Maximum).Maximum
    }
} | ConvertTo-Json | Set-Content -Encoding utf8 -LiteralPath (Join-Path $OutputDirectory 'summary.json')
