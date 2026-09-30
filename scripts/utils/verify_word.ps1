param(
    [Parameter(Mandatory)][string]$Source,
    [string]$Translated,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [switch]$CreateFixture,
    [string]$Image
)

$ErrorActionPreference = 'Stop'
$SourcePath = [IO.Path]::GetFullPath($Source)
$OutputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
$Word = New-Object -ComObject Word.Application
$Word.Visible = $false
$Word.DisplayAlerts = 0
$Word.AutomationSecurity = 3
try {
    if ($CreateFixture) {
        if (Test-Path -LiteralPath $SourcePath) { throw "La muestra ya existe: $SourcePath" }
        $ImagePath = (Resolve-Path -LiteralPath $Image).Path
        $Document = $Word.Documents.Add()
        try {
            $Document.Content.Font.Name = 'Calibri'
            $Document.Content.Font.Size = 11
            $Document.Content.Text = "Equipment inspection report`rCheck the critical components before starting.`rInspection results`r"
            $Document.Paragraphs.Item(1).Range.Font.Size = 20
            $Document.Paragraphs.Item(1).Range.Font.Bold = -1
            $FindRange = $Document.Content.Duplicate
            $FindRange.Find.Text = 'critical'
            if ($FindRange.Find.Execute()) { $FindRange.Font.Bold = -1; $FindRange.Font.Color = 255 }
            $EndRange = $Document.Range($Document.Content.End - 1, $Document.Content.End - 1)
            $Table = $Document.Tables.Add($EndRange, 4, 3)
            $Table.Borders.Enable = -1
            $Table.AllowAutoFit = $false
            $Table.Columns.Width = 145
            $Table.Cell(1, 1).Merge($Table.Cell(1, 3))
            $Table.Cell(1, 1).Range.Text = 'Inspection checklist'
            $Table.Cell(1, 1).Range.Font.Bold = -1
            $Table.Cell(1, 1).Shading.BackgroundPatternColor = 15132390
            foreach ($Cell in @(
                @(2, 1, 'Component'), @(2, 2, 'Status'), @(2, 3, 'Observation'),
                @(3, 1, 'Safety guard'), @(3, 2, 'Ready'), @(3, 3, 'No visible damage'),
                @(4, 1, 'Power cable'), @(4, 2, 'Replace'), @(4, 3, 'Disconnect before inspection')
            )) { $Table.Cell($Cell[0], $Cell[1]).Range.Text = $Cell[2] }
            $EndRange = $Document.Range($Document.Content.End - 1, $Document.Content.End - 1)
            $EndRange.InsertAfter("`rReference photograph`r")
            $EndRange = $Document.Range($Document.Content.End - 1, $Document.Content.End - 1)
            $Picture = $Document.InlineShapes.AddPicture($ImagePath, $false, $true, $EndRange)
            $Picture.Width = 90
            $Picture.Height = 90
            $EndRange = $Document.Range($Document.Content.End - 1, $Document.Content.End - 1)
            $EndRange.InsertAfter("`rDetail photographs`r")
            $PhotoTable = $Document.Tables.Add($Document.Range($Document.Content.End - 1, $Document.Content.End - 1), 2, 2)
            $PhotoTable.Borders.Enable = -1
            $PhotoTable.AllowAutoFit = $false
            $PhotoTable.Columns.Width = 217.5
            $PhotoTable.Cell(1, 1).Range.Text = 'Front view'
            $PhotoTable.Cell(1, 2).Range.Text = 'Rear view'
            foreach ($Column in @(1, 2)) {
                $CellRange = $PhotoTable.Cell(2, $Column).Range.Duplicate
                $CellRange.End -= 2
                $CellRange.Collapse(0)
                $Picture = $Document.InlineShapes.AddPicture($ImagePath, $false, $true, $CellRange)
                $Picture.Width = 65
                $Picture.Height = 65
            }
            $EndRange = $Document.Range($Document.Content.End - 1, $Document.Content.End - 1)
            $EndRange.InsertAfter("`rMaintenance instructions`rDisconnect the equipment and verify that the indicator is off.`rRecord the inspection results and report any damage.`r")
            $FindRange = $Document.Content.Duplicate
            $FindRange.Find.Text = 'Maintenance instructions'
            if ($FindRange.Find.Execute()) { $FindRange.ParagraphFormat.PageBreakBefore = -1; $FindRange.Font.Bold = -1 }
            $Document.SaveAs2($SourcePath, 16)
        } finally { $Document.Close(0) }
    }

    $Reports = @()
    foreach ($Entry in @(@{ Name = 'source'; Path = $SourcePath }, @{ Name = 'translated'; Path = $Translated })) {
        if (-not $Entry.Path) { continue }
        $Path = (Resolve-Path -LiteralPath $Entry.Path).Path
        $HashBefore = (Get-FileHash -LiteralPath $Path).Hash
        $Document = $Word.Documents.Open($Path, $false, $true, $false)
        try {
            $Document.Repaginate()
            $Tables = @()
            foreach ($Table in $Document.Tables) {
                $Tables += @{ Rows = $Table.Rows.Count; Cells = $Table.Range.Cells.Count; Widths = @($Table.Range.Cells | ForEach-Object { [math]::Round($_.Width, 2) }) }
            }
            $Pictures = @($Document.InlineShapes | ForEach-Object { @{ Width = [math]::Round($_.Width, 2); Height = [math]::Round($_.Height, 2); Type = $_.Type } })
            $PdfPath = Join-Path $OutputPath ($Entry.Name + '.pdf')
            $Document.ExportAsFixedFormat($PdfPath, 17)
            $Reports += @{
                Name = $Entry.Name; Path = $Path; SHA256 = $HashBefore
                WordVersion = $Word.Version; WordBuild = $Word.Build
                Pages = $Document.ComputeStatistics(2); Paragraphs = $Document.Paragraphs.Count
                Sections = $Document.Sections.Count; Tables = $Tables; Pictures = $Pictures
                FloatingShapes = $Document.Shapes.Count; Text = $Document.Content.Text
                PDF = $PdfPath
            }
        } finally { $Document.Close(0) }
        if ((Get-FileHash -LiteralPath $Path).Hash -ne $HashBefore) { throw "Word modificó el archivo de prueba: $Path" }
    }
    if ($Reports.Count -eq 2) {
        foreach ($Property in @('Tables', 'Pictures', 'FloatingShapes', 'Sections')) {
            if (($Reports[0][$Property] | ConvertTo-Json -Depth 8 -Compress) -ne ($Reports[1][$Property] | ConvertTo-Json -Depth 8 -Compress)) {
                throw "El Word traducido cambió la estructura o geometría de $Property"
            }
        }
        if ($Reports[1].Text -match '<[gx](?:\s|>)') { throw 'Se filtraron códigos del editor al Word exportado' }
    }
    $Reports | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputPath 'word-report.json') -Encoding utf8
    $Reports | Select-Object Name, Pages, Paragraphs, FloatingShapes, WordVersion, WordBuild
} finally {
    $Word.Quit()
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Word)
}
