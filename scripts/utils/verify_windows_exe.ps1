param(
    [string[]]$Paths = @('output/lumencat.exe', 'output/lumencat-gpui.exe')
)

$ErrorActionPreference = 'Stop'
foreach ($Path in $Paths) {
    $Resolved = (Resolve-Path -LiteralPath $Path).Path
    $Bytes = [IO.File]::ReadAllBytes($Resolved)
    if ($Bytes.Length -lt 64 -or [BitConverter]::ToUInt16($Bytes, 0) -ne 0x5A4D) {
        throw "No es un ejecutable PE: $Resolved"
    }
    $PeOffset = [BitConverter]::ToInt32($Bytes, 0x3C)
    if ($PeOffset -lt 0 -or $PeOffset + 94 -gt $Bytes.Length -or [BitConverter]::ToUInt32($Bytes, $PeOffset) -ne 0x4550) {
        throw "Cabecera PE inválida: $Resolved"
    }
    $Subsystem = [BitConverter]::ToUInt16($Bytes, $PeOffset + 24 + 68)
    if ($Subsystem -ne 2) {
        throw "El EXE puede abrir consola: se esperaba Windows GUI (2), se obtuvo $Subsystem en $Resolved"
    }
    [pscustomobject]@{ Path = $Resolved; Subsystem = 'Windows GUI'; SHA256 = (Get-FileHash -LiteralPath $Resolved).Hash }
}
