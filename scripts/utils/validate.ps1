param([switch]$Release)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $projectRoot
try {
    foreach ($check in @(@('fmt','--check'), @('clippy','--locked','--all-targets','--','-D','warnings'), @('test','--locked'))) {
        & cargo @check
        if ($LASTEXITCODE -ne 0) { throw "Falló cargo $($check -join ' ')" }
    }
    if ($Release) {
        & cargo build --locked --release --bin lumencat --bin benchmark
        if ($LASTEXITCODE -ne 0) { throw 'Falló build release' }
    }
} finally { Pop-Location }
