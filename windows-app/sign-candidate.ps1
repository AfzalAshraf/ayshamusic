# Sign a locally built Aysha executable with a certificate already installed in
# the current user's Windows certificate store. No certificate or password is
# imported, uploaded, or stored by this script.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-fA-F0-9]{40}$')]
    [string] $CertificateThumbprint,
    [Parameter(Mandatory = $true)]
    [string] $SignToolPath,
    [Parameter(Mandatory = $true)]
    [string] $TimestampUrl,
    [string] $InputExe = "$PSScriptRoot/target/release/aysha-windows.exe",
    [string] $OutputDirectory = "$PSScriptRoot/signed-candidate"
)

$ErrorActionPreference = 'Stop'
$timestamp = $null
if (-not [Uri]::TryCreate($TimestampUrl, [UriKind]::Absolute, [ref]$timestamp) -or
    $timestamp.Scheme -notin @('https', 'http') -or -not $timestamp.Host -or
    $timestamp.UserInfo -or $timestamp.Query -or $timestamp.Fragment) {
    throw 'TimestampUrl must be a plain HTTP(S) timestamp service URL without credentials, query, or fragment.'
}
if (-not (Test-Path -LiteralPath $SignToolPath -PathType Leaf)) { throw 'SignTool.exe was not found.' }
if (-not (Test-Path -LiteralPath $InputExe -PathType Leaf)) { throw 'Build the Windows candidate before signing.' }
$certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$CertificateThumbprint" -ErrorAction Stop
if (-not $certificate.HasPrivateKey) { throw 'The selected certificate has no accessible private key.' }
if ($certificate.NotBefore -gt (Get-Date) -or $certificate.NotAfter -le (Get-Date)) { throw 'The certificate is not currently valid.' }
if (-not ($certificate.EnhancedKeyUsageList | Where-Object { $_.ObjectId.Value -eq '1.3.6.1.5.5.7.3.3' })) { throw 'The certificate does not list code signing as an intended use.' }

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$outputExe = Join-Path $OutputDirectory 'aysha-windows.exe'
if ([System.IO.Path]::GetFullPath($InputExe) -eq [System.IO.Path]::GetFullPath($outputExe)) { throw 'Input and output must be different files.' }
if (Test-Path -LiteralPath $outputExe) { throw 'Output executable already exists; choose an empty output folder.' }
Copy-Item -LiteralPath $InputExe -Destination $outputExe
try {
    & $SignToolPath sign /s My /sha1 $CertificateThumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $outputExe
    if ($LASTEXITCODE -ne 0) { throw 'Signing or timestamping failed.' }
    & $SignToolPath verify /pa /all /v $outputExe
    if ($LASTEXITCODE -ne 0) { throw 'Windows signature verification failed.' }
} catch {
    Remove-Item -LiteralPath $outputExe -Force -ErrorAction SilentlyContinue
    throw
}
Write-Host 'Signed and verified the Aysha executable. Keep the packaged yt-dlp.exe and deno.exe alongside it.'
Write-Host 'Signature verification does not guarantee an Application Control policy will permit this or the helper executables.'
