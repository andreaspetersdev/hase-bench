[CmdletBinding()]
param(
    [switch]$Recreate
)

$ErrorActionPreference = "Stop"
$ExpectedPythonVersion = "3.12.4"
$PipVersion = "24.0"
$SetuptoolsVersion = "69.5.1"
$WheelVersion = "0.43.0"
$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$VenvRoot = Join-Path $RepositoryRoot ".venv"
$VenvPython = Join-Path $VenvRoot "Scripts\\python.exe"

function Get-ExactPython {
    $versionCode = "import sys; print('.'.join(map(str, sys.version_info[:3])))"
    $pyLauncher = Get-Command py.exe -ErrorAction SilentlyContinue
    if ($null -ne $pyLauncher) {
        $version = (& $pyLauncher.Source -3.12 -c $versionCode).Trim()
        if ($LASTEXITCODE -eq 0 -and $version -eq $ExpectedPythonVersion) {
            $path = (& $pyLauncher.Source -3.12 -c "import sys; print(sys.executable)").Trim()
            if ($LASTEXITCODE -eq 0 -and $path) { return $path }
        }
    }
    $python = Get-Command python.exe -ErrorAction SilentlyContinue
    if ($null -ne $python) {
        $version = (& $python.Source -c $versionCode).Trim()
        if ($LASTEXITCODE -eq 0 -and $version -eq $ExpectedPythonVersion) { return $python.Source }
    }
    throw "Python $ExpectedPythonVersion is required. Install that exact version, ensure 'py -3.12' or 'python.exe' resolves to it, then run this script again."
}

function Assert-ExactPython([string]$PythonPath, [string]$Description) {
    $version = (& $PythonPath -c "import sys; print('.'.join(map(str, sys.version_info[:3])))").Trim()
    if ($LASTEXITCODE -ne 0 -or $version -ne $ExpectedPythonVersion) {
        throw "$Description must use Python $ExpectedPythonVersion; found '$version'. Remove .venv and rerun .\\tools\\setup-env.ps1."
    }
}

Push-Location $RepositoryRoot
try {
    if ($Recreate -and (Test-Path -LiteralPath $VenvRoot)) {
        Write-Host "Removing existing .venv for a clean rebuild."
        Remove-Item -LiteralPath $VenvRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $VenvPython) {
        Assert-ExactPython $VenvPython "Existing .venv"
        Write-Host "Using existing .venv with Python $ExpectedPythonVersion."
    }
    elseif (Test-Path -LiteralPath $VenvRoot) {
        throw "Existing .venv is incomplete. Remove .venv and rerun .\\tools\\setup-env.ps1."
    }
    else {
        $PythonPath = Get-ExactPython
        Write-Host "Creating .venv with Python $ExpectedPythonVersion ($PythonPath)."
        & $PythonPath -m venv $VenvRoot
        if ($LASTEXITCODE -ne 0) { throw "Failed to create .venv." }
        Assert-ExactPython $VenvPython "New .venv"
    }
    & $VenvPython -m pip install --upgrade "pip==$PipVersion" "setuptools==$SetuptoolsVersion" "wheel==$WheelVersion"
    if ($LASTEXITCODE -ne 0) { throw "Failed to install pinned packaging tools." }
    & $VenvPython -m pip install --editable ".[dev]"
    if ($LASTEXITCODE -ne 0) { throw "Failed to install hase-bench and pinned development dependencies." }
    & $VenvPython -m hasebench list
    if ($LASTEXITCODE -ne 0) { throw "hase-bench sanity check failed." }
    & $VenvPython -m pytest -q
    if ($LASTEXITCODE -ne 0) { throw "pytest sanity check failed." }
    Write-Host ""
    Write-Host "Environment ready. Activate it with:"
    Write-Host "  .\\.venv\\Scripts\\Activate.ps1"
}
finally { Pop-Location }
