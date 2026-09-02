param(
    [Parameter(Mandatory = $true)]
    [string] $TesmioLoaderRoot,

    [Parameter(Mandatory = $true)]
    [string] $OutputRoot
)

$ErrorActionPreference = 'Stop'
$sourceRoot = (Resolve-Path -LiteralPath $TesmioLoaderRoot).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$loaderSource = Join-Path $sourceRoot 'src\tesmioloader.cpp'
$launcherSource = Join-Path $sourceRoot 'src\tesmiolauncher.cpp'
$launcherResourceSource = Join-Path $sourceRoot 'src\tesmiolauncher.rc'

foreach ($required in @($loaderSource, $launcherSource, $launcherResourceSource)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "The reviewed TesmioLoader source is incomplete: $([System.IO.Path]::GetFileName($required))."
    }
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw 'Visual Studio Build Tools with the Desktop development with C++ workload are required.'
}
$installation = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $installation) { throw 'No compatible Visual C++ toolchain was found.' }
$vcvars = Join-Path $installation 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path -LiteralPath $vcvars -PathType Leaf)) {
    throw 'The Visual C++ x64 environment could not be found.'
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
$loaderDll = Join-Path $outputRoot 'tesmioloader.dll'
$loaderObject = Join-Path $outputRoot 'tesmioloader.obj'
$loaderImportLibrary = Join-Path $outputRoot 'tesmioloader.lib'
$loaderCommand = 'call "{0}" >nul && cl /nologo /O2 /MT /W3 /EHsc /LD /Fo"{1}" /I"{2}\src" "{3}" /link /OUT:"{4}" /IMPLIB:"{5}" kernel32.lib' -f $vcvars, $loaderObject, $sourceRoot, $loaderSource, $loaderDll, $loaderImportLibrary
& $env:ComSpec /d /s /c $loaderCommand
if ($LASTEXITCODE -ne 0) { throw "TesmioLoader compilation failed with exit code $LASTEXITCODE." }

$launcherExe = Join-Path $outputRoot 'tesmiolauncher.exe'
$launcherObject = Join-Path $outputRoot 'tesmiolauncher.obj'
$launcherResource = Join-Path $outputRoot 'tesmiolauncher.res'
$resourceCommand = 'call "{0}" >nul && rc /nologo /fo "{1}" "{2}"' -f $vcvars, $launcherResource, $launcherResourceSource
& $env:ComSpec /d /s /c $resourceCommand
$resourceArgument = if ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $launcherResource -PathType Leaf)) { '"{0}"' -f $launcherResource } else { '' }
$launcherCommand = 'call "{0}" >nul && cl /nologo /O2 /MT /W3 /EHsc /Fo"{1}" /I"{2}\src" "{3}" {4} /link /OUT:"{5}" /SUBSYSTEM:WINDOWS /MANIFEST:EMBED kernel32.lib' -f $vcvars, $launcherObject, $sourceRoot, $launcherSource, $resourceArgument, $launcherExe
& $env:ComSpec /d /s /c $launcherCommand
if ($LASTEXITCODE -ne 0) { throw "TesmioLauncher compilation failed with exit code $LASTEXITCODE." }

Write-Host "Built reviewed observation host: $loaderDll"
Write-Host "Built reviewed observation launcher: $launcherExe"
Write-Host 'No game or save file was changed by this build step.'
