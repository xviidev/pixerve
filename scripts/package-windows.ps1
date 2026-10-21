[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$VipsDir,
    [string]$Binary,
    [string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
if (-not $Binary) { $Binary = Join-Path $project 'target/release/pixerve.exe' }
if (-not $OutputDir) { $OutputDir = Join-Path $project 'dist' }
$sdk = (Resolve-Path -LiteralPath $VipsDir).Path
$exe = (Resolve-Path -LiteralPath $Binary).Path
$versions = Get-Content -LiteralPath (Join-Path $sdk 'versions.json') -Raw | ConvertFrom-Json
$versionMatch = [regex]::Match((Get-Content -LiteralPath (Join-Path $project 'Cargo.toml') -Raw), '(?m)^version = "([^"]+)"')
if (-not $versionMatch.Success) { throw 'Cargo package version not found' }
$name = 'pixerve-v' + $versionMatch.Groups[1].Value + '-windows-x64'
$null = New-Item -ItemType Directory -Path $OutputDir -Force
$output = (Resolve-Path -LiteralPath $OutputDir).Path
$stage = Join-Path $output $name
if (Test-Path -LiteralPath $stage) { throw "Package directory already exists: $stage. Use another OutputDir for a rebuild." }
$null = New-Item -ItemType Directory -Path $stage
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'pixerve.exe')
$dlls = @(Get-ChildItem -LiteralPath (Join-Path $sdk 'bin') -Filter '*.dll' -File)
if (-not ($dlls.Name -contains 'libvips-42.dll')) { throw 'SDK does not contain libvips-42.dll' }
foreach ($dll in $dlls) { Copy-Item -LiteralPath $dll.FullName -Destination $stage }
foreach ($folder in @('etc','share')) {
    $source = Join-Path $sdk $folder
    if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination $stage -Recurse }
}
$modules = @(Get-ChildItem -LiteralPath (Join-Path $sdk 'lib') -Directory -Filter 'vips-modules-*')
if ($modules.Count) {
    $null = New-Item -ItemType Directory -Path (Join-Path $stage 'lib')
    foreach ($module in $modules) { Copy-Item -LiteralPath $module.FullName -Destination (Join-Path $stage 'lib') -Recurse }
}
foreach ($folder in @('assets','static')) {
    Copy-Item -LiteralPath (Join-Path $project "$folder") -Destination $stage -Recurse
}
foreach ($item in @('docs','CDN.md','CONTRIBUTING.md')) { Copy-Item -LiteralPath (Join-Path $project $item) -Destination $stage -Recurse }
$notice = Join-Path $stage 'third-party/libvips'
$null = New-Item -ItemType Directory -Path $notice -Force
foreach ($item in @('LICENSE','README.md','versions.json')) {
    Copy-Item -LiteralPath (Join-Path $sdk $item) -Destination $notice
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
$example = Get-Content -LiteralPath (Join-Path $project '.env.example') -Raw
$example = $example.Replace('# SOURCE_DIR=static','SOURCE_DIR=static').Replace('# Relative to the repository root; Docker overrides both.','# Paths are relative to this portable package directory.')
[IO.File]::WriteAllText((Join-Path $stage '.env.example'), $example, $utf8)
$portable = @'
# Pixerve portable for Windows x64

This package includes Pixerve, libvips runtime DLLs, a neutral sample image, and
a transparent starter OG overlay. No Rust compiler or libvips SDK is required.
Extract the entire directory; keep its DLLs beside pixerve.exe.

From PowerShell in this directory:

```powershell
Copy-Item .env.example .env
./pixerve.exe
```

Open http://localhost:3000/health and use
http://localhost:3000/example.png?w=640&f=webp (follow the canonical redirect).
The initial configuration uses local sample storage and conservative open limits.
Configure signing and a trusted TLS proxy before production use.

For S3, clear SOURCE_DIR and fill in S3 settings. Replace assets/og_wm.png with
your own overlay to add visible branding; the starter overlay is transparent.
Run from this directory so configuration and relative paths resolve correctly.

See [documentation](docs/README.md), [configuration](docs/configuration.md), and
[CDN delivery](CDN.md). Commands in source-build guides assume a source checkout;
in this package use ./pixerve.exe instead of ./target/release/pixerve.

manifest.json records bundled file hashes and native dependency versions.
third-party/libvips contains the upstream SDK license and version information.
The application distribution license is not yet selected. This is a local build,
not evidence of a published GitHub release.
'@
[IO.File]::WriteAllText((Join-Path $stage 'README.md'), $portable, $utf8)
$entries = @(Get-ChildItem -LiteralPath $stage -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path=$_.FullName.Substring($stage.Length+1).Replace('\','/'); bytes=$_.Length; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$manifest = [ordered]@{ name='pixerve'; version=$versionMatch.Groups[1].Value; target='x86_64-pc-windows-msvc'; nativeVersions=$versions; nativeSource='https://github.com/libvips/build-win64-mxe'; files=$entries }
[IO.File]::WriteAllText((Join-Path $stage 'manifest.json'), ($manifest | ConvertTo-Json -Depth 8), $utf8)
$archive = Join-Path $output "$name.zip"
# tar includes .env.example consistently on Windows, unlike Compress-Archive.
& tar.exe -a -c -f $archive -C $output $name
if ($LASTEXITCODE -ne 0) { throw 'ZIP creation failed' }
$checksum = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$archive.sha256", "$checksum  $name.zip`n", $utf8)
Write-Output "Created $archive"
