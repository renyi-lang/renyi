# Install the renyi binary from the GitHub Release of a version (decision
# AI2), on Windows. Usage, in PowerShell:
#   irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex
# Environment: RENYI_VERSION (a tag such as v0.1.0; the latest release when
# unset), RENYI_INSTALL_DIR (where the binary goes; the user's local
# programs directory when unset), RENYI_REPO (renyi-lang/renyi when unset).
$ErrorActionPreference = "Stop"

$repo = if ($env:RENYI_REPO) { $env:RENYI_REPO } else { "renyi-lang/renyi" }
# the directory is read from the environment at run time
$installDir = if ($env:RENYI_INSTALL_DIR) { $env:RENYI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\renyi" }

if ([System.Environment]::Is64BitOperatingSystem -eq $false) {
    throw "install.ps1: no binary for 32-bit Windows; `cargo install renyi` builds one"
}
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne "AMD64") {
    throw "install.ps1: no binary for Windows on $arch yet; `cargo install renyi` builds one"
}
$target = "x86_64-pc-windows-msvc"

$version = $env:RENYI_VERSION
if (-not $version) {
    $latest = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -Headers @{ "User-Agent" = "renyi-install" }
    $version = $latest.tag_name
    if (-not $version) { throw "install.ps1: could not read the latest release of $repo" }
}

$name = "renyi-$version-$target"
$url = "https://github.com/$repo/releases/download/$version/$name.zip"
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("renyi-install-" + [System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Force $work | Out-Null
try {
    Write-Host "downloading $url"
    Invoke-WebRequest -Uri $url -OutFile (Join-Path $work "$name.zip")
    Invoke-WebRequest -Uri "$url.sha256" -OutFile (Join-Path $work "$name.zip.sha256")

    $expected = (Get-Content (Join-Path $work "$name.zip.sha256") -Raw).Split(" ")[0].Trim().ToLower()
    $actual = (Get-FileHash (Join-Path $work "$name.zip") -Algorithm SHA256).Hash.ToLower()
    if ($expected -ne $actual) { throw "install.ps1: the checksum does not match" }

    Expand-Archive -Path (Join-Path $work "$name.zip") -DestinationPath $work -Force
    New-Item -ItemType Directory -Force $installDir | Out-Null
    Copy-Item (Join-Path $work "$name\renyi.exe") (Join-Path $installDir "renyi.exe") -Force
    Write-Host "installed $(& (Join-Path $installDir 'renyi.exe') --version) to $installDir\renyi.exe"
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}

$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if (-not (($userPath -split ";") -contains $installDir)) {
    [System.Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host "added $installDir to the user PATH; open a new terminal to use `renyi`"
}
