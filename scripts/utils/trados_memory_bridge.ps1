param(
    [Parameter(Mandatory)][ValidateSet('export', 'update')][string]$Action,
    [Parameter(Mandatory)][string]$StudioDirectory,
    [Parameter(Mandatory)][string]$WorkingMemory,
    [Parameter(Mandatory)][string]$Tmx,
    [Parameter(Mandatory)][string]$Report,
    [Parameter(Mandatory)][string]$SourceLanguage,
    [Parameter(Mandatory)][string]$TargetLanguage
)

$ErrorActionPreference = 'Stop'
$result = @{ source_lang = ''; target_lang = ''; unit_count = 0; read = 0; imported = 0; added = 0; overwritten = 0; merged = 0; discarded = 0; errors = 0; bad = 0; duplicates = 0; error = $null }
try {
    $workspace = Split-Path -Parent $WorkingMemory
    if ((Split-Path -Leaf $WorkingMemory) -ne 'working.sdltm' -or -not (Test-Path -LiteralPath (Join-Path $workspace '.lumencat-sdltm-workspace'))) {
        $result.error = 'private_snapshot_required'
        throw 'El puente solo acepta una copia privada creada por LumenCAT'
    }
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Reflection;
public static class LumenTradosAssemblyLoader {
    private static string directory;
    public static void Register(string path) {
        directory = path;
        AppDomain.CurrentDomain.AssemblyResolve += Resolve;
    }
    private static Assembly Resolve(object sender, ResolveEventArgs args) {
        var path = Path.Combine(directory, new AssemblyName(args.Name).Name + ".dll");
        return File.Exists(path) ? Assembly.LoadFrom(path) : null;
    }
}
'@
    [LumenTradosAssemblyLoader]::Register($StudioDirectory)
    foreach ($name in @('Sdl.Core.Globalization.dll', 'Sdl.LanguagePlatform.Core.dll', 'Sdl.LanguagePlatform.TranslationMemory.dll', 'Sdl.LanguagePlatform.TranslationMemoryApi.dll')) {
        [void][Reflection.Assembly]::LoadFrom((Join-Path $StudioDirectory $name))
    }
    $memory = New-Object Sdl.LanguagePlatform.TranslationMemoryApi.FileBasedTranslationMemory($WorkingMemory)
    $result.source_lang = $memory.LanguageDirection.SourceLanguage.Name
    $result.target_lang = $memory.LanguageDirection.TargetLanguage.Name
    if ($result.source_lang -ine $SourceLanguage -or $result.target_lang -ine $TargetLanguage) {
        $result.error = 'language_mismatch'
        throw 'Par de idiomas incompatible'
    }
    if ($Action -eq 'export') {
        $exporter = New-Object Sdl.LanguagePlatform.TranslationMemoryApi.TranslationMemoryExporter($memory.LanguageDirection)
        $exporter.Export($Tmx, $false)
    } else {
        $importer = New-Object Sdl.LanguagePlatform.TranslationMemoryApi.TranslationMemoryImporter($memory.LanguageDirection)
        $settings = $importer.ImportSettings
        $settings.PlainText = $false
        $settings.CheckMatchingSublanguages = $true
        $settings.ExistingTUsUpdateMode = [Enum]::Parse($settings.GetType().GetProperty('ExistingTUsUpdateMode').PropertyType, 'Overwrite')
        $settings.ExistingFieldsUpdateMode = [Enum]::Parse($settings.GetType().GetProperty('ExistingFieldsUpdateMode').PropertyType, 'Merge')
        $importer.Import($Tmx)
        $stats = $importer.Statistics
        $result.read = $stats.TotalRead
        $result.imported = $stats.TotalImported
        $result.added = $stats.AddedTranslationUnits
        $result.overwritten = $stats.OverwrittenTranslationUnits
        $result.merged = $stats.MergedTranslationUnits
        $result.discarded = $stats.DiscardedTranslationUnits
        $result.errors = $stats.Errors
        $result.bad = $stats.BadTranslationUnits
        $result.duplicates = $stats.DuplicateTranslationUnits
        if ($stats.Errors -gt 0 -or $stats.BadTranslationUnits -gt 0 -or $stats.DiscardedTranslationUnits -gt $stats.DuplicateTranslationUnits -or $stats.TranslationUnitsOverEntitlementLimits -gt 0) {
            $result.error = 'import_rejected_units'
            throw 'Importación incompleta; no publicar la copia'
        }
        $memory.Save()
    }
    $result.unit_count = $memory.GetTranslationUnitCount()
    [IO.File]::WriteAllText($Report, ($result | ConvertTo-Json -Compress), (New-Object Text.UTF8Encoding($false)))
    exit 0
} catch {
    if (-not $result.error) { $result.error = 'sdk_api_failed' }
    [IO.File]::WriteAllText($Report, ($result | ConvertTo-Json -Compress), (New-Object Text.UTF8Encoding($false)))
    exit 1
}
