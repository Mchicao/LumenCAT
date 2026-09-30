param([int[]]$Counts = @(10000,100000,1000000))
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $projectRoot
try {
    & cargo build --locked --release --bin benchmark
    if ($LASTEXITCODE -ne 0) { throw 'Falló build benchmark' }
    $runStamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    New-Item -ItemType Directory -Force output,logs/tests | Out-Null
    foreach ($unitCount in $Counts) {
        if ($unitCount -lt 1 -or $unitCount -gt 5000000) { throw 'Cantidad fuera de 1..5000000' }
        & .cache/target/release/benchmark.exe $unitCount "output/benchmark-$runStamp-$unitCount.csv" 2> "logs/tests/benchmark-$runStamp-$unitCount.log"
        if ($LASTEXITCODE -ne 0) { throw "Falló benchmark $unitCount; revisar log" }
    }
} finally { Pop-Location }
