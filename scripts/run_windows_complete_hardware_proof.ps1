[CmdletBinding()]
param(
    [string]$ProofRoot = (Join-Path $HOME "scena-gpu-proof\scena-windows-browser-proof"),
    [string]$NodeRoot = (Join-Path $HOME "scena-gpu-proof\node-v20.20.0-win-x64"),
    [string]$BrowserExecutable = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    [Parameter(Mandatory = $true)]
    [string]$UploadUrl
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$bundleRoot = $PSScriptRoot
$manifestPath = Join-Path $bundleRoot "bundle-files.sha256"

function Get-ManifestEntries {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Proof bundle manifest is missing: $Path"
    }
    $entries = @()
    foreach ($line in Get-Content -LiteralPath $Path) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        if ($line -notmatch '^([0-9a-fA-F]{64})  (.+)$') {
            throw "Malformed proof bundle manifest line: $line"
        }
        $entries += [pscustomobject]@{
            Hash = $Matches[1].ToUpperInvariant()
            Path = $Matches[2]
        }
    }
    if ($entries.Count -eq 0) {
        throw "Proof bundle manifest is empty"
    }
    return $entries
}

function Assert-ManifestAtRoot {
    param(
        [object[]]$Entries,
        [string]$Root,
        [string]$Label
    )

    foreach ($entry in $Entries) {
        $relative = $entry.Path.Replace('/', [IO.Path]::DirectorySeparatorChar)
        $file = Join-Path $Root $relative
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw "$Label is missing manifest file: $($entry.Path)"
        }
        $actual = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
        if ($actual -ne $entry.Hash) {
            throw "$Label SHA-256 mismatch for $($entry.Path): expected $($entry.Hash), got $actual"
        }
    }
}

function Invoke-Checked {
    param(
        [string]$Label,
        [scriptblock]$Command
    )

    Write-Host "proof-step: $Label"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

$manifest = @(Get-ManifestEntries -Path $manifestPath)
Assert-ManifestAtRoot -Entries $manifest -Root $bundleRoot -Label "Downloaded proof bundle"

if (-not (Test-Path -LiteralPath $ProofRoot -PathType Container)) {
    throw "Existing Windows proof workspace is missing: $ProofRoot"
}
if (-not (Test-Path -LiteralPath $NodeRoot -PathType Container)) {
    throw "Pinned portable Node installation is missing: $NodeRoot"
}
if (-not (Test-Path -LiteralPath $BrowserExecutable -PathType Leaf)) {
    throw "Hardware browser executable is missing: $BrowserExecutable"
}
if (-not (Test-Path -LiteralPath (Join-Path $ProofRoot "node_modules\playwright") -PathType Container)) {
    throw "The existing proof workspace has no Playwright installation"
}

$targetRoot = Join-Path $ProofRoot "target"
$pf01Package = Join-Path $targetRoot "pf01-output-toggle-browser-pkg"
$fr06Package = Join-Path $targetRoot "fr06-semantic-aov-browser-pkg"
$browserDir = Join-Path $ProofRoot "tests\browser"
$releaseDir = Join-Path $ProofRoot "tests\release"
$assetDir = Join-Path $ProofRoot "tests\assets\gltf"
$binDir = Join-Path $ProofRoot "bin"

foreach ($directory in @($pf01Package, $fr06Package, $browserDir, $releaseDir, $assetDir, $binDir)) {
    Remove-Item -LiteralPath $directory -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
}

Copy-Item -Path (Join-Path $bundleRoot "target\pf01-output-toggle-browser-pkg\*") `
    -Destination $pf01Package -Recurse -Force
Copy-Item -Path (Join-Path $bundleRoot "target\pf01-output-toggle-browser-pkg\*") `
    -Destination $fr06Package -Recurse -Force
Copy-Item -Path (Join-Path $bundleRoot "tests\browser\*") `
    -Destination $browserDir -Recurse -Force
Copy-Item -Path (Join-Path $bundleRoot "tests\release\*") `
    -Destination $releaseDir -Recurse -Force
Copy-Item -Path (Join-Path $bundleRoot "tests\assets\gltf\*") `
    -Destination $assetDir -Recurse -Force
Copy-Item -Path (Join-Path $bundleRoot "bin\*") `
    -Destination $binDir -Recurse -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "package.json") -Destination $ProofRoot -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "AGENTS.md") -Destination $ProofRoot -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "run-proof.ps1") -Destination $ProofRoot -Force

$proofSkills = Join-Path $ProofRoot ".codex\skills"
Remove-Item -LiteralPath $proofSkills -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Split-Path $proofSkills) | Out-Null
Copy-Item -LiteralPath (Join-Path $bundleRoot ".codex\skills") `
    -Destination $proofSkills -Recurse -Force

Assert-ManifestAtRoot -Entries $manifest -Root $ProofRoot -Label "Installed proof workspace"

$env:PATH = "$NodeRoot;$env:PATH"
$nodeVersion = (& node --version).Trim()
$npmVersion = (& npm.cmd --version).Trim()
if ($nodeVersion -ne "v20.20.0") {
    throw "Expected Node v20.20.0, got $nodeVersion"
}
if ($npmVersion -ne "10.8.2") {
    throw "Expected npm 10.8.2, got $npmVersion"
}

$gateRoot = Join-Path $targetRoot "gate-artifacts"
foreach ($relative in @(
    "pf01-output-toggle",
    "pf01-pf02-native-surface",
    "fr06-semantic-aov",
    "windows-complete-hardware-proof"
)) {
    Remove-Item -LiteralPath (Join-Path $gateRoot $relative) -Recurse -Force -ErrorAction SilentlyContinue
}
$runRoot = Join-Path $gateRoot "windows-complete-hardware-proof"
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
$transcriptPath = Join-Path $runRoot "run.log"
$summaryPath = Join-Path $runRoot "proof-summary.json"
$metadataPath = Join-Path $runRoot "execution-metadata.json"
$installedManifest = Join-Path $runRoot "input-bundle-files.sha256"
Copy-Item -LiteralPath $manifestPath -Destination $installedManifest -Force

$bundleManifestHash = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash
$nativeExe = Join-Path $binDir "scena-native-hardware-proof.exe"
$nativeFr06Exe = Join-Path $binDir "scena-fr06-native-hardware-proof.exe"
$failure = $null
$transcriptStarted = $false

try {
    Start-Transcript -LiteralPath $transcriptPath -Force | Out-Null
    $transcriptStarted = $true
    Set-Location $ProofRoot

    $env:SCENA_SKIP_WASM_BUILD = "1"
    $env:SCENA_REQUIRE_PARITY = "1"
    $env:SCENA_BROWSER_BACKENDS = "webgpu,webgl2"
    $env:SCENA_WEBGPU_BROWSER = "chromium"
    $env:SCENA_WEBGL2_BROWSER = "chromium"
    $env:SCENA_BROWSER_EXECUTABLE = $BrowserExecutable
    Remove-Item Env:SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS -ErrorAction SilentlyContinue
    Remove-Item Env:SCENA_PF01_ASSET_URL -ErrorAction SilentlyContinue

    Invoke-Checked "strict GPU/PF01 evaluator tests" {
        & npm.cmd run test:required-gpu-parity
    }
    Invoke-Checked "complete hardware-proof validator tests" {
        & node .\tests\release\windows_complete_hardware_proof_validation_test.js
    }
    Invoke-Checked "strict combined WebGPU/WebGL2 PF01 browser proof" {
        & npm.cmd run browser:pf01-output-toggle
    }
    Invoke-Checked "strict combined WebGPU/WebGL2 FR06 browser proof" {
        & npm.cmd run browser:fr06-semantic-aov
    }

    $env:SCENA_HARDWARE_PROOF_ROOT = $ProofRoot
    $env:SCENA_REQUIRE_HARDWARE_GPU = "1"
    $env:SCENA_HARDWARE_PROOF_COMMAND = ".\bin\scena-native-hardware-proof.exe"
    Invoke-Checked "attached native PF01/PF02 hardware proof" {
        & $nativeExe
    }

    $nativeFr06Test = "fr06_headless_gpu_semantic_aov_matches_cpu_center_truth"
    $env:SCENA_HARDWARE_PROOF_COMMAND = ".\bin\scena-fr06-native-hardware-proof.exe --exact $nativeFr06Test --nocapture"
    Invoke-Checked "native FR06 semantic-AOV hardware proof" {
        & $nativeFr06Exe --exact $nativeFr06Test --nocapture
    }

    Invoke-Checked "independent complete artifact validation" {
        & node .\tests\release\windows_complete_hardware_proof_validation.js $ProofRoot $summaryPath
    }
}
catch {
    $failure = $_
    Write-Host "proof-failure: $($_.Exception.Message)"
}
finally {
    if ($transcriptStarted) {
        Stop-Transcript | Out-Null
    }
}

$executionStatus = if ($null -eq $failure) { "passed" } else { "failed" }
[ordered]@{
    schema = "scena.windows_complete_hardware_proof_execution.v1"
    generated_at = [DateTime]::UtcNow.ToString("o")
    status = $executionStatus
    failure = if ($null -eq $failure) { $null } else { $failure.Exception.Message }
    node_version = $nodeVersion
    npm_version = $npmVersion
    browser_executable = $BrowserExecutable
    bundle_manifest_sha256 = $bundleManifestHash
    native_executable_sha256 = (Get-FileHash -LiteralPath $nativeExe -Algorithm SHA256).Hash
    native_fr06_executable_sha256 = (Get-FileHash -LiteralPath $nativeFr06Exe -Algorithm SHA256).Hash
    wasm_sha256 = (Get-FileHash -LiteralPath (Join-Path $pf01Package "scena_bg.wasm") -Algorithm SHA256).Hash
} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $metadataPath -Encoding UTF8

$uploadRoot = Join-Path $targetRoot "windows-complete-hardware-proof-upload"
$archivePath = Join-Path $targetRoot "scena-windows-complete-hardware-proof.zip"
Remove-Item -LiteralPath $uploadRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $archivePath -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $uploadRoot | Out-Null
if (Test-Path -LiteralPath $gateRoot -PathType Container) {
    Copy-Item -LiteralPath $gateRoot -Destination (Join-Path $uploadRoot "gate-artifacts") -Recurse -Force
}
Copy-Item -LiteralPath $manifestPath -Destination (Join-Path $uploadRoot "input-bundle-files.sha256") -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "run-proof.ps1") -Destination $uploadRoot -Force
Compress-Archive -Path (Join-Path $uploadRoot "*") -DestinationPath $archivePath -CompressionLevel Optimal

$archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
$upload = Invoke-WebRequest -UseBasicParsing -Method Put -Uri $UploadUrl -InFile $archivePath
if ($upload.StatusCode -ne 201) {
    throw "Complete proof archive upload failed with HTTP $($upload.StatusCode)"
}

if ($null -ne $failure) {
    throw "Complete hardware proof failed, but its diagnostic archive uploaded successfully (SHA-256 $archiveHash): $($failure.Exception.Message)"
}

[ordered]@{
    status = "PASSED"
    release_evidence = $true
    browser_backends = @("webgpu", "webgl2")
    native_surface = $true
    native_semantic_aov = $true
    summary = $summaryPath
    archive = $archivePath
    archive_sha256 = $archiveHash
    upload_status = $upload.StatusCode
} | ConvertTo-Json -Depth 8
