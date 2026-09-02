param(
    [Parameter(Mandatory = $true)]
    [string] $TesmioLoaderRoot
)

$ErrorActionPreference = 'Stop'
$probeRoot = $PSScriptRoot
$tesmioRoot = (Resolve-Path -LiteralPath $TesmioLoaderRoot).Path
$header = Join-Path $tesmioRoot 'src\tesmio_plugin.h'
if (-not (Test-Path -LiteralPath $header -PathType Leaf)) {
    throw 'The selected TesmioLoader checkout does not contain src\tesmio_plugin.h.'
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw 'Visual Studio Build Tools with the Desktop development with C++ workload are required.'
}
$installation = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $installation) { throw 'No compatible Visual C++ toolchain was found.' }
$vcvars = Join-Path $installation 'VC\Auxiliary\Build\vcvars64.bat'
$output = Join-Path $probeRoot 'build'
New-Item -ItemType Directory -Force -Path $output | Out-Null
$source = Join-Path $probeRoot 'observatory_probe.cpp'
$dll = Join-Path $output 'observatory_probe.dll'
$object = Join-Path $output 'observatory_probe.obj'
$importLibrary = Join-Path $output 'observatory_probe.lib'
$command = 'call "{0}" && cl /nologo /LD /O2 /MT /W4 /WX /wd4505 /EHsc /Fo"{3}" /I"{1}\src" "{2}" /link /OUT:"{4}" /IMPLIB:"{5}"' -f $vcvars, $tesmioRoot, $source, $object, $dll, $importLibrary
& $env:ComSpec /d /s /c $command
if ($LASTEXITCODE -ne 0) { throw "Probe compilation failed with exit code $LASTEXITCODE." }
Copy-Item -LiteralPath (Join-Path $probeRoot 'observatory_probe.ini') -Destination (Join-Path $output 'observatory_probe.ini') -Force
Write-Host "Built $dll"
Write-Host 'Nothing was installed into the game. Observatory can prepare the checked session after separate consent.'
