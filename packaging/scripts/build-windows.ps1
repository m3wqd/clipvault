$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "../..")
function Run-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}
foreach ($tool in @("node", "npm", "cargo", "rustup")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Missing $tool. Install Node.js LTS and Rust stable, plus Visual Studio C++ Build Tools. See README."
    }
}
Run-Checked "npm" @("ci")
Run-Checked "npm" @("run", "build")
Run-Checked "npm" @("test")
Run-Checked "cargo" @("fmt", "--manifest-path", "src-tauri/Cargo.toml", "--check")
Run-Checked "cargo" @("clippy", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets", "--locked", "--", "-D", "warnings")
Run-Checked "cargo" @("test", "--manifest-path", "src-tauri/Cargo.toml", "--locked")
Run-Checked "npm" @("run", "tauri", "build", "--", "--target", "x86_64-pc-windows-msvc", "--bundles", "nsis,msi")
$bundle = "src-tauri/target/x86_64-pc-windows-msvc/release/bundle"
$exe = @(Get-ChildItem "$bundle/nsis/*-setup.exe")
$msi = @(Get-ChildItem "$bundle/msi/*.msi")
if ($exe.Count -eq 0 -or $msi.Count -eq 0) { throw "Required installers are missing" }
New-Item -ItemType Directory -Force -Path "dist-installers" | Out-Null
$exe + $msi | Copy-Item -Destination "dist-installers"
Get-ChildItem "dist-installers" -File | Where-Object Extension -In @(".exe", ".msi") | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "$hash  $($_.Name)" | Add-Content "dist-installers/SHA256SUMS.txt"
}
Write-Host "Built real Windows installers in dist-installers. They are unsigned unless signing is configured."
