param(
    [Parameter(Mandatory = $true)]
    [string] $TesmioBuildRoot
)

$ErrorActionPreference = 'Stop'
$resolvedRoot = (Resolve-Path -LiteralPath $TesmioBuildRoot).Path
$configurationPath = Join-Path $resolvedRoot 'tesmioloader.ini'
$pluginRoot = Join-Path $resolvedRoot 'plugins'

if (-not (Test-Path -LiteralPath $configurationPath -PathType Leaf)) {
    throw 'Observation-only verification failed: tesmioloader.ini is missing.'
}
if (-not (Test-Path -LiteralPath $pluginRoot -PathType Container)) {
    throw 'Observation-only verification failed: the plugins directory is missing.'
}

$configurationFile = Get-Item -LiteralPath $configurationPath -Force
if ($configurationFile.Length -gt 65536) {
    throw 'Observation-only verification failed: tesmioloader.ini exceeds 64 KiB.'
}

$rawConfiguration = Get-Content -LiteralPath $configurationPath -Raw
if ($rawConfiguration.StartsWith([char]0xFEFF)) {
    throw 'Observation-only verification failed: tesmioloader.ini has a BOM that upstream warns may break parsing.'
}

$sections = @{}
$sectionName = ''
$lineNumber = 0
foreach ($rawLine in ($rawConfiguration -split "`r?`n")) {
    $lineNumber++
    $line = $rawLine.Trim()
    if (-not $line -or $line.StartsWith(';') -or $line.StartsWith('#')) { continue }
    if ($line -match '^\[([^\]]+)\]$') {
        $sectionName = $Matches[1].Trim().ToLowerInvariant()
        if (-not $sections.ContainsKey($sectionName)) { $sections[$sectionName] = @{} }
        continue
    }
    if (-not $sectionName -or $line -notmatch '^([^=]+)=(.*)$') {
        throw "Observation-only verification failed: unsupported INI syntax on line $lineNumber."
    }
    $key = $Matches[1].Trim().ToLowerInvariant()
    $value = $Matches[2].Trim()
    if ($sections[$sectionName].ContainsKey($key)) {
        throw "Observation-only verification failed: duplicate [$sectionName] $key setting."
    }
    $sections[$sectionName][$key] = $value
}

$requiredHostSettings = [ordered]@{
    trace_reads = '0'
    log_game = '0'
    vfs = '0'
    probe_map = '0'
    probe_texel = '0'
    save_manifest = '0'
    plugins = '1'
    menu_patch = '0'
    version_check = '1'
}

if (-not $sections.ContainsKey('tesmioloader')) {
    throw 'Observation-only verification failed: [tesmioloader] is missing.'
}
foreach ($entry in $requiredHostSettings.GetEnumerator()) {
    $actual = $sections['tesmioloader'][$entry.Key]
    if ($actual -ne $entry.Value) {
        throw "Observation-only verification failed: [tesmioloader] $($entry.Key) must be $($entry.Value), found '$actual'."
    }
}

if (-not $sections.ContainsKey('plugins') -or $sections['plugins']['observatory_probe'] -ne '1') {
    throw 'Observation-only verification failed: [plugins] observatory_probe must be 1.'
}

$pluginDlls = @(Get-ChildItem -LiteralPath $pluginRoot -File -Filter '*.dll' | Sort-Object Name)
if ($pluginDlls.Count -ne 1 -or $pluginDlls[0].Name -ine 'observatory_probe.dll') {
    $found = if ($pluginDlls.Count) { ($pluginDlls.Name -join ', ') } else { 'none' }
    throw "Observation-only verification failed: plugins must contain only observatory_probe.dll; found $found."
}

$probeConfigurationPath = Join-Path $pluginRoot 'observatory_probe.ini'
if (-not (Test-Path -LiteralPath $probeConfigurationPath -PathType Leaf)) {
    throw 'Observation-only verification failed: observatory_probe.ini is missing.'
}

Write-Host 'Observation-only TesmioLoader verification passed.'
Write-Host 'The reviewed profile disables save manifests, VFS, built-in memory probes, game-log mirroring, menu changes, and version bypass.'
Write-Host 'Only observatory_probe.dll is present. This verifies configuration, not operating-system sandboxing or absence of native-code risk.'
