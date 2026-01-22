# Orbital OS Build Script for Windows
# Usage: .\build.ps1 [target]
#   targets: all, web, processes, clean

param(
    [string]$Target = "all"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot

function Write-Step($message) {
    Write-Host "`n=== $message ===" -ForegroundColor Cyan
}

function Build-WebModules {
    Write-Step "Building supervisor and desktop WASM modules"
    
    $configPath = "$ProjectRoot\.cargo\config.toml"
    $configBackup = "$ProjectRoot\.cargo\config.toml.bak"
    $hasConfig = Test-Path $configPath
    
    try {
        # Temporarily disable threading config (only needed for process binaries)
        if ($hasConfig) {
            Write-Host "Temporarily disabling .cargo/config.toml (threading flags)"
            Move-Item $configPath $configBackup -Force
        }
        
        # Build orbital-web
        Write-Host "Building orbital-web..."
        Push-Location "$ProjectRoot\crates\orbital-web"
        wasm-pack build --target web --out-dir ../../web/pkg
        if ($LASTEXITCODE -ne 0) { throw "orbital-web build failed" }
        Pop-Location
        
        # Build orbital-desktop
        Write-Host "Building orbital-desktop..."
        Push-Location "$ProjectRoot\crates\orbital-desktop"
        wasm-pack build --target web --features wasm
        if ($LASTEXITCODE -ne 0) { throw "orbital-desktop build failed" }
        Pop-Location
        
        # Copy desktop pkg to web folder
        Write-Host "Copying orbital-desktop to web/pkg-desktop..."
        if (-not (Test-Path "$ProjectRoot\web\pkg-desktop")) {
            New-Item -ItemType Directory -Path "$ProjectRoot\web\pkg-desktop" | Out-Null
        }
        Copy-Item -Recurse -Force "$ProjectRoot\crates\orbital-desktop\pkg\*" "$ProjectRoot\web\pkg-desktop\"
        
        Write-Host "Web modules built successfully!" -ForegroundColor Green
    }
    finally {
        # Always restore the config
        if ($hasConfig -and (Test-Path $configBackup)) {
            Move-Item $configBackup $configPath -Force
            Write-Host "Restored .cargo/config.toml"
        }
    }
}

function Build-Processes {
    Write-Step "Building process WASM binaries (with threading support)"
    
    Push-Location $ProjectRoot
    try {
        # Build init
        Write-Host "Building orbital-init..."
        cargo +nightly build -p orbital-init --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
        if ($LASTEXITCODE -ne 0) { throw "orbital-init build failed" }
        
        # Build test processes
        Write-Host "Building orbital-test-procs..."
        cargo +nightly build -p orbital-test-procs --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
        if ($LASTEXITCODE -ne 0) { throw "orbital-test-procs build failed" }
        
        # Build apps
        Write-Host "Building orbital-apps..."
        cargo +nightly build -p orbital-apps --bins --target wasm32-unknown-unknown --release -Z build-std=std,panic_abort
        if ($LASTEXITCODE -ne 0) { throw "orbital-apps build failed" }
        
        # Copy to web/processes
        Write-Host "Copying process binaries to web/processes..."
        if (-not (Test-Path "$ProjectRoot\web\processes")) {
            New-Item -ItemType Directory -Path "$ProjectRoot\web\processes" | Out-Null
        }
        
        $releaseDir = "$ProjectRoot\target\wasm32-unknown-unknown\release"
        Copy-Item "$releaseDir\orbital_init.wasm" "$ProjectRoot\web\processes\init.wasm" -Force
        Copy-Item "$releaseDir\terminal.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\permission_manager.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\idle.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\memhog.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\sender.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\receiver.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\pingpong.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\clock.wasm" "$ProjectRoot\web\processes\" -Force
        Copy-Item "$releaseDir\calculator.wasm" "$ProjectRoot\web\processes\" -Force
        
        Write-Host "Process binaries built successfully!" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}

function Start-DevServer {
    Write-Step "Starting development server"
    Push-Location $ProjectRoot
    cargo run -p dev-server
    Pop-Location
}

function Clean-Build {
    Write-Step "Cleaning build artifacts"
    Push-Location $ProjectRoot
    cargo clean
    Remove-Item -Recurse -Force "$ProjectRoot\web\pkg" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "$ProjectRoot\web\pkg-desktop" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "$ProjectRoot\web\processes" -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force "$ProjectRoot\crates\orbital-desktop\pkg" -ErrorAction SilentlyContinue
    Pop-Location
    Write-Host "Clean complete!" -ForegroundColor Green
}

# Main
switch ($Target.ToLower()) {
    "all" {
        Build-Processes
        Build-WebModules
        Write-Host "`nBuild complete! Run '.\build.ps1 dev' or 'cargo run -p dev-server' to start." -ForegroundColor Green
    }
    "web" {
        Build-WebModules
    }
    "processes" {
        Build-Processes
    }
    "dev" {
        Build-Processes
        Build-WebModules
        Start-DevServer
    }
    "clean" {
        Clean-Build
    }
    default {
        Write-Host "Orbital OS Build Script"
        Write-Host ""
        Write-Host "Usage: .\build.ps1 [target]"
        Write-Host ""
        Write-Host "Targets:"
        Write-Host "  all        - Build everything (default)"
        Write-Host "  web        - Build only supervisor/desktop WASM modules"
        Write-Host "  processes  - Build only process WASM binaries"
        Write-Host "  dev        - Build all and start dev server"
        Write-Host "  clean      - Clean all build artifacts"
    }
}
