$ErrorActionPreference = "Stop"

cargo build --release

$bin = "target\release\vibetone.exe"
$size = (Get-Item $bin).Length / 1MB
Write-Host "Built: $bin ($([math]::Round($size, 1)) MB)"

# Install to Program Files
$dest = "$env:LOCALAPPDATA\Vibetone"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
Copy-Item $bin "$dest\vibetone.exe"

# Create Start Menu shortcut
$startMenu = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut("$startMenu\Vibetone.lnk")
$shortcut.TargetPath = "$dest\vibetone.exe"
$shortcut.WorkingDirectory = $dest
$shortcut.Description = "Real-time sidetone for vibe coders"
$shortcut.Save()

Write-Host "Installed to $dest\vibetone.exe"
Write-Host "Added to Start Menu"
Write-Host "Run from Start Menu or:  & '$dest\vibetone.exe'"
