param(
  [Parameter(Mandatory = $true)][string]$Browser,
  [Parameter(Mandatory = $true)][string]$Flavor,
  [Parameter(Mandatory = $true)][string]$ExtensionId,
  [string]$InstallRoot = "$env:LOCALAPPDATA\Keystone"
)

$ErrorActionPreference = "Stop"

if ($Flavor -notin @("dev", "beta", "prod")) {
  throw "Invalid flavor: $Flavor"
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SourceBinary = Join-Path $ScriptDir "keystone.exe"

if (-not (Test-Path $SourceBinary)) {
  throw "keystone.exe not found next to this script: $SourceBinary"
}

$TargetDir = Join-Path $InstallRoot $Flavor
New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null

$TargetBinary = Join-Path $TargetDir "keystone.exe"
Copy-Item $SourceBinary $TargetBinary -Force

$ReadmePath = Join-Path $ScriptDir "README.md"
if (Test-Path $ReadmePath) {
  Copy-Item $ReadmePath (Join-Path $TargetDir "README.md") -Force
}

$InstallerPath = Join-Path $ScriptDir "INSTALLER.md"
if (Test-Path $InstallerPath) {
  Copy-Item $InstallerPath (Join-Path $TargetDir "INSTALLER.md") -Force
}

Write-Host "installed binary: $TargetBinary"
Write-Host "registering browser manifest for target: $Browser"
& $TargetBinary install $Browser $Flavor $ExtensionId $TargetBinary
Write-Host "done"
Write-Host "next step: reload the extension and click 'Test Keystone Connection' in Y/TXT Options."
