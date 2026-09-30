param(
    [Parameter(Mandatory)][string]$Pdf,
    [Parameter(Mandatory)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Storage.Streams.IRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
$asTaskOperation = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
})[0]
$asTaskAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'
})[0]
function Invoke-Await($WinRtTask, $ResultType) {
    $task = $asTaskOperation.MakeGenericMethod($ResultType).Invoke($null, @($WinRtTask))
    $task.Wait(-1) | Out-Null
    $task.Result
}
function Invoke-AwaitAction($WinRtTask) {
    $task = $asTaskAction.Invoke($null, @($WinRtTask))
    $task.Wait(-1) | Out-Null
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$pdfPath = (Resolve-Path -LiteralPath $Pdf).Path
$file = Invoke-Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($pdfPath)) ([Windows.Storage.StorageFile])
$document = Invoke-Await ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
$folder = Invoke-Await ([Windows.Storage.StorageFolder]::GetFolderFromPathAsync((Resolve-Path -LiteralPath $OutputDirectory).Path)) ([Windows.Storage.StorageFolder])
for ($pageNumber = 0; $pageNumber -lt $document.PageCount; $pageNumber++) {
    $page = $document.GetPage([uint32]$pageNumber)
    $target = Invoke-Await ($folder.CreateFileAsync(("page-{0}.png" -f ($pageNumber + 1)), [Windows.Storage.CreationCollisionOption]::ReplaceExisting)) ([Windows.Storage.StorageFile])
    $stream = Invoke-Await ($target.OpenAsync([Windows.Storage.FileAccessMode]::ReadWrite)) ([Windows.Storage.Streams.IRandomAccessStream])
    try {
        $options = New-Object Windows.Data.Pdf.PdfPageRenderOptions
        $options.DestinationWidth = [uint32]1200
        Invoke-AwaitAction ($page.RenderToStreamAsync($stream, $options))
    } finally {
        $stream.Dispose()
        $page.Dispose()
    }
}
"Renderizadas $($document.PageCount) páginas de $pdfPath"
