$ErrorActionPreference = "Stop"

cargo build --release

$bin = "target\release\vibetone.exe"
$size = (Get-Item $bin).Length / 1MB
Write-Host "Built: $bin ($([math]::Round($size, 1)) MB)"

$dest = "$env:USERPROFILE\.cargo\bin\vibetone.exe"
Copy-Item $bin $dest
Write-Host "Installed to $dest"
Write-Host "Run from anywhere:  vibetone"
