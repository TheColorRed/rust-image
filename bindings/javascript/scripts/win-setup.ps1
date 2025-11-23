param(
    [string]$PythonPath = ""
)

# PowerShell script to temporarily prepare the environment and run `npm install` on Windows.
Try {
    Write-Host "Running Windows setup script..." -ForegroundColor Cyan

    # Ensure we are using cmd.exe semantics for msbuild
    $env:COMSPEC = "C:\\Windows\\System32\\cmd.exe"
    # Unset MSYS/Git shell env vars which can interfere with MSBuild quoting and cmd semantics
    $env:SHELL = $null
    $env:MSYSTEM = $null
    $env:MSYS = $null
    $env:MSYS2_ARG_CONV_EXCL = $null
    $env:CHERE_INVOKING = $null

    # Remove MSYS/Git mingw entries from PATH temporarily for this shell
    $paths = $env:PATH -split ';' | Where-Object { $_ -and ($_ -notmatch 'msys64') -and ($_ -notmatch 'MINGW64') -and ($_ -notmatch 'Git\\mingw64') }
    $env:PATH = ($paths -join ';')

    # Try to discover a Windows Python on PATH (avoid msys/git mingw)
    function Find-WindowsPython {
        $candidates = @()
        try {
            $out = & where.exe python 2>$null
            if ($out) { $candidates += ($out -split '\r?\n' | Where-Object { $_ -ne '' }) }
        } catch {}
        # prefer Python installed in AppData or Program Files instead of MSYS/Git or Windows Store shim
        $winCandidate = $candidates | Where-Object { $_ -match 'AppData|Program Files' -and $_ -notmatch 'msys64|MINGW|WindowsApps' } | Select-Object -First 1
        if ($winCandidate) { return $winCandidate }
        # fallback: first candidate not containing msys
        $fallback = $candidates | Where-Object { $_ -notmatch 'msys64|MINGW|Git\\mingw64|WindowsApps' } | Select-Object -First 1
        if ($fallback) { return $fallback }
        return $null
    }

    $detectedPython = $null
    if ([string]::IsNullOrEmpty($PythonPath)) { $detectedPython = Find-WindowsPython } else { if (Test-Path $PythonPath) { $detectedPython = $PythonPath } }
    # Validate candidate by running --version if it's present
    if ($detectedPython) {
        try {
            $ver = & "$detectedPython" --version 2>$null
            if (-not $ver) { throw "NoVersion" }
        } catch {
            Write-Warning "Detected Python '$detectedPython' is not callable; please install CPython and add it to PATH."
            $detectedPython = $null
        }
    }
    if ($detectedPython) {
        $env:PYTHON = $detectedPython
        $env:npm_config_python = $detectedPython
        Write-Host "Using Python:" $detectedPython -ForegroundColor Green
    } else {
        Write-Warning "Could not locate a Windows CPython on PATH. Please install CPython from python.org and add it to PATH, or pass -PythonPath to this script."
    }

    # Run from project directory if the script lives in scripts
    Push-Location (Split-Path -Parent $MyInvocation.MyCommand.Path)
    if (Test-Path "..\\package.json") { Push-Location ".." }

    # Cleanup node_modules then install with verbose logs
    if (Test-Path node_modules) {
        try { npx rimraf node_modules } catch { Remove-Item -Recurse -Force node_modules -ErrorAction SilentlyContinue }
    }
    npm cache clean --force
    # Write a debugging snapshot to help diagnose quoting/ENV issues
    try {
        $debuglog = Join-Path (Get-Location) '.setup-env-debug.log'
        "---DEBUG ENV SNAPSHOT---`n" | Out-File -FilePath $debuglog -Append
        "COMSPEC=$env:COMSPEC`nPYTHON=$env:PYTHON`nSHELL=$env:SHELL`nMSYSTEM=$env:MSYSTEM`nMSYS=$env:MSYS`nMSYS2_ARG_CONV_EXCL=$env:MSYS2_ARG_CONV_EXCL`nCHERE_INVOKING=$env:CHERE_INVOKING`n`n" | Out-File -FilePath $debuglog -Append
    } catch { }
    npm i --verbose
    # If installation errors, capture the environment for debugging
    if ($LASTEXITCODE -ne 0) {
        Write-Host "npm install returned exit code $LASTEXITCODE" -ForegroundColor Yellow
        Write-Host "Diagnostic environment vars: `nCOMSPEC=$env:COMSPEC`nPYTHON=$env:PYTHON`nnpm_config_python=$env:npm_config_python" -ForegroundColor Yellow
    }
    Pop-Location
} Catch {
    Write-Error $_.Exception.Message
    Exit 1
}
