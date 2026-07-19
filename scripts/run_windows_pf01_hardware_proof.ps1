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
$phaseIds = @("off", "bloom_only", "fxaa_only", "on", "off_again")

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

function Resource-Signature {
    param($Value)
    return ($Value | ConvertTo-Json -Compress -Depth 8)
}

function Assert-Report {
    param($Report)

    if ($Report.schema -ne "scena.pf01.browser_output_toggle.v1") {
        throw "Unexpected PF01 artifact schema: $($Report.schema)"
    }
    if ($Report.status -ne "passed") {
        throw "PF01 artifact status is not passed: $($Report.status)"
    }
    if ($Report.release_evidence -ne $true) {
        throw "PF01 artifact is not release evidence"
    }
    if ($Report.required_hardware -ne $true) {
        throw "PF01 artifact did not require hardware"
    }
    if ($Report.complete_backend_set -ne $true) {
        throw "PF01 artifact does not contain the complete backend set"
    }

    $backends = @($Report.backends)
    if ($backends.Count -ne 2) {
        throw "PF01 artifact must contain exactly two backends; got $($backends.Count)"
    }
    $names = @($backends | ForEach-Object { $_.backend } | Sort-Object)
    if (($names -join ',') -ne "webgl2,webgpu") {
        throw "PF01 artifact backend set is invalid: $($names -join ',')"
    }

    foreach ($backend in $backends) {
        $name = $backend.backend
        if ($backend.hardware_evidence.status -ne "passed") {
            throw "$name hardware evidence failed: $($backend.hardware_evidence | ConvertTo-Json -Compress -Depth 8)"
        }
        if ($backend.browser_engine -ne "chromium") {
            throw "$name used an unexpected browser engine: $($backend.browser_engine)"
        }
        if ($backend.browser_gpu.source -ne "chromium-cdp-system-info") {
            throw "$name lacks CDP GPU attestation"
        }

        $phases = $backend.phases
        foreach ($id in $phaseIds) {
            $phase = $phases.$id
            if ($null -eq $phase -or $phase.id -ne $id) {
                throw "$name is missing PF01 phase $id"
            }
            if ([int64]$phase.nonblack -le 0) {
                throw "$name $id output is blank"
            }
            if ([string]::IsNullOrWhiteSpace([string]$phase.fnv1a64)) {
                throw "$name $id output has no pixel hash"
            }
            if ((Resource-Signature $phase.resources_before_render) -ne
                (Resource-Signature $phase.resources_after_render)) {
                throw "$name $id changed prepared resources during render"
            }
        }

        $off = $phases.off
        $bloomOnly = $phases.bloom_only
        $fxaaOnly = $phases.fxaa_only
        $on = $phases.on
        $offAgain = $phases.off_again

        if ($bloomOnly.fnv1a64 -eq $off.fnv1a64) {
            throw "$name bloom-only output collapsed to baseline"
        }
        if ($fxaaOnly.fnv1a64 -eq $off.fnv1a64) {
            throw "$name FXAA-only output collapsed to baseline"
        }
        if ($on.fnv1a64 -eq $off.fnv1a64) {
            throw "$name combined output collapsed to baseline"
        }
        if ($on.fnv1a64 -eq $bloomOnly.fnv1a64) {
            throw "$name combined output collapsed to bloom-only"
        }
        if ($on.fnv1a64 -eq $fxaaOnly.fnv1a64) {
            throw "$name combined output collapsed to FXAA-only"
        }
        if ($offAgain.fnv1a64 -ne $off.fnv1a64) {
            throw "$name off-again output did not restore the baseline pixels"
        }

        $offResources = Resource-Signature $off.resources_before_render
        foreach ($enabled in @($bloomOnly, $fxaaOnly, $on)) {
            if ((Resource-Signature $enabled.resources_before_render) -eq $offResources) {
                throw "$name $($enabled.id) did not prepare a distinct resource shape"
            }
        }
        if ((Resource-Signature $offAgain.resources_before_render) -ne $offResources) {
            throw "$name off-again did not restore the baseline resource shape"
        }
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

$targetPackage = Join-Path $ProofRoot "target\pf01-output-toggle-browser-pkg"
Remove-Item -LiteralPath $targetPackage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Split-Path $targetPackage) | Out-Null
Copy-Item -LiteralPath (Join-Path $bundleRoot "target\pf01-output-toggle-browser-pkg") `
    -Destination $targetPackage -Recurse -Force

$browserDir = Join-Path $ProofRoot "tests\browser"
$assetDir = Join-Path $ProofRoot "tests\assets\gltf"
New-Item -ItemType Directory -Force -Path $browserDir, $assetDir | Out-Null
Get-ChildItem -LiteralPath (Join-Path $bundleRoot "tests\browser") -File | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $browserDir -Force
}
Copy-Item -LiteralPath (Join-Path $bundleRoot "tests\assets\gltf\exploded_view_assembly.gltf") `
    -Destination $assetDir -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "package.json") -Destination $ProofRoot -Force
Copy-Item -LiteralPath (Join-Path $bundleRoot "AGENTS.md") -Destination $ProofRoot -Force

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

Set-Location $ProofRoot
$env:SCENA_SKIP_WASM_BUILD = "1"
$env:SCENA_REQUIRE_PARITY = "1"
$env:SCENA_BROWSER_BACKENDS = "webgpu,webgl2"
$env:SCENA_WEBGPU_BROWSER = "chromium"
$env:SCENA_WEBGL2_BROWSER = "chromium"
$env:SCENA_BROWSER_EXECUTABLE = $BrowserExecutable
Remove-Item Env:SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS -ErrorAction SilentlyContinue
Remove-Item Env:SCENA_PF01_ASSET_URL -ErrorAction SilentlyContinue

& npm.cmd run test:required-gpu-parity
if ($LASTEXITCODE -ne 0) {
    throw "Strict GPU/PF01 evaluator tests failed"
}

& npm.cmd run browser:pf01-output-toggle
if ($LASTEXITCODE -ne 0) {
    throw "Combined strict WebGPU/WebGL2 PF01 proof failed"
}

$artifact = Join-Path $ProofRoot "target\gate-artifacts\pf01-output-toggle\browser\browser-output-toggle.json"
if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
    throw "PF01 artifact was not written: $artifact"
}
$report = Get-Content -LiteralPath $artifact -Raw | ConvertFrom-Json
Assert-Report -Report $report

$artifactHash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash
$upload = Invoke-WebRequest -UseBasicParsing -Method Put -Uri $UploadUrl -InFile $artifact
if ($upload.StatusCode -ne 201) {
    throw "PF01 artifact upload failed with HTTP $($upload.StatusCode)"
}

$hashes = @{}
foreach ($backend in @($report.backends)) {
    $hashes[$backend.backend] = [ordered]@{
        off = $backend.phases.off.fnv1a64
        bloom_only = $backend.phases.bloom_only.fnv1a64
        fxaa_only = $backend.phases.fxaa_only.fnv1a64
        on = $backend.phases.on.fnv1a64
        off_again = $backend.phases.off_again.fnv1a64
    }
}

[ordered]@{
    status = "PASSED"
    release_evidence = $true
    complete_backend_set = $true
    artifact = $artifact
    artifact_sha256 = $artifactHash
    upload_status = $upload.StatusCode
    phase_hashes = $hashes
} | ConvertTo-Json -Depth 8
