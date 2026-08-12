param(
  [Parameter(Mandatory = $true)]
  [string]$Repository,

  [Parameter(Mandatory = $true)]
  [string]$Tag,

  [Parameter(Mandatory = $true)]
  [string]$ExpectedCommit,

  [switch]$Publish
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$version = $Tag.TrimStart('v')
$releaseList = gh api "repos/$Repository/releases?per_page=100" | ConvertFrom-Json
$matches = @($releaseList | Where-Object tag_name -eq $Tag)
if ($matches.Count -ne 1) {
  throw "Expected exactly one release for $Tag, found $($matches.Count)."
}

$release = $matches[0]
if (-not $release.draft) {
  throw "Release $Tag must remain a draft until aggregate verification succeeds."
}
if ($release.target_commitish -ne $ExpectedCommit) {
  throw "Release targets $($release.target_commitish), expected $ExpectedCommit."
}

$platformArtifacts = [ordered]@{
  'windows-x86_64' = "OpenCarpanel_${version}_windows_x64_nsis-setup.exe"
  'darwin-aarch64' = "OpenCarpanel_${version}_darwin_aarch64_app.app.tar.gz"
  'darwin-x86_64' = "OpenCarpanel_${version}_darwin_x64_app.app.tar.gz"
}
$installerAssets = @(
  "OpenCarpanel_${version}_windows_x64_msi.msi",
  "OpenCarpanel_${version}_darwin_aarch64_dmg.dmg",
  "OpenCarpanel_${version}_darwin_x64_dmg.dmg"
)
$requiredAssets = @('latest.json') + $installerAssets
foreach ($artifactName in $platformArtifacts.Values) {
  $requiredAssets += $artifactName
  $requiredAssets += "$artifactName.sig"
}
$requiredAssets += "OpenCarpanel_${version}_windows_x64_msi.msi.sig"

$assetByName = @{}
foreach ($asset in $release.assets) {
  if ($assetByName.ContainsKey($asset.name)) {
    throw "Release contains duplicate asset $($asset.name)."
  }
  $assetByName[$asset.name] = $asset
}
foreach ($assetName in $requiredAssets) {
  if (-not $assetByName.ContainsKey($assetName)) {
    throw "Release is missing required asset $assetName."
  }
  $asset = $assetByName[$assetName]
  if ($asset.state -ne 'uploaded' -or $asset.size -le 0) {
    throw "Release asset $assetName is not a complete non-empty upload."
  }
}

$unexpectedAssets = @($assetByName.Keys | Where-Object { $_ -notin $requiredAssets })
if ($unexpectedAssets.Count -gt 0) {
  throw "Release contains unexpected assets: $($unexpectedAssets -join ', ')."
}

$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) "opencarpanel-release-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $temporaryDirectory | Out-Null
try {
  $manifestPath = Join-Path $temporaryDirectory 'latest.json'
  $manifestText = gh api -H 'Accept: application/octet-stream' $assetByName['latest.json'].url | Out-String
  $manifest = $manifestText | ConvertFrom-Json
  if ($manifest.version -ne $version) {
    throw "Updater manifest version $($manifest.version) does not match $version."
  }

  $normalizedPlatforms = [ordered]@{}
  foreach ($entry in $platformArtifacts.GetEnumerator()) {
    $platformName = $entry.Key
    $artifactName = $entry.Value
    $platform = $manifest.platforms.PSObject.Properties[$platformName].Value
    if ($null -eq $platform -or [string]::IsNullOrWhiteSpace($platform.signature)) {
      throw "Updater manifest is missing a signature for $platformName."
    }

    $signatureAsset = $assetByName["$artifactName.sig"]
    $signatureText = (
      gh api -H 'Accept: application/octet-stream' $signatureAsset.url | Out-String
    ).Trim()
    if ($signatureText -ne $platform.signature) {
      throw "Updater manifest signature does not match $artifactName.sig."
    }

    $normalizedPlatforms[$platformName] = [ordered]@{
      signature = $platform.signature
      url = "https://github.com/$Repository/releases/download/$Tag/$artifactName"
    }
  }

  $normalizedManifest = [ordered]@{
    version = $manifest.version
    notes = $manifest.notes
    pub_date = $manifest.pub_date
    platforms = $normalizedPlatforms
  }
  $normalizedManifest |
    ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath $manifestPath -Encoding utf8NoBOM

  $roundTrip = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
  foreach ($platformName in $platformArtifacts.Keys) {
    $platform = $roundTrip.platforms.PSObject.Properties[$platformName].Value
    if ($null -eq $platform) {
      throw "Normalized updater manifest lost $platformName."
    }
  }

  if (-not $Publish) {
    Write-Output "Verified draft $Tag for $ExpectedCommit; publication was not requested."
    return
  }

  gh release upload $Tag $manifestPath --repo $Repository --clobber

  $updatedList = gh api "repos/$Repository/releases?per_page=100" | ConvertFrom-Json
  $updatedRelease = @($updatedList | Where-Object tag_name -eq $Tag)[0]
  $updatedManifestAsset = @($updatedRelease.assets | Where-Object name -eq 'latest.json')[0]
  $uploadedManifest = (
    gh api -H 'Accept: application/octet-stream' $updatedManifestAsset.url | Out-String
  ) | ConvertFrom-Json
  foreach ($platformName in $platformArtifacts.Keys) {
    $platform = $uploadedManifest.platforms.PSObject.Properties[$platformName].Value
    $expectedUrl = "https://github.com/$Repository/releases/download/$Tag/$($platformArtifacts[$platformName])"
    if ($platform.url -ne $expectedUrl -or [string]::IsNullOrWhiteSpace($platform.signature)) {
      throw "Uploaded updater entry for $platformName failed draft round-trip verification."
    }
  }

  gh release edit $Tag --repo $Repository --draft=false --latest

  $published = $null
  for ($attempt = 1; $attempt -le 5; $attempt += 1) {
    try {
      $published = gh api "repos/$Repository/releases/tags/$Tag" | ConvertFrom-Json
      break
    } catch {
      if ($attempt -eq 5) { throw }
      Start-Sleep -Seconds 3
    }
  }
  if ($published.draft -or $published.tag_name -ne $Tag) {
    throw "Release $Tag did not become public after publication."
  }

  $publicManifestText = $null
  for ($attempt = 1; $attempt -le 5; $attempt += 1) {
    try {
      $publicManifestText = Invoke-RestMethod -Uri "https://github.com/$Repository/releases/download/$Tag/latest.json"
      break
    } catch {
      if ($attempt -eq 5) { throw }
      Start-Sleep -Seconds 3
    }
  }
  if ($publicManifestText -is [string]) {
    $publicManifest = $publicManifestText | ConvertFrom-Json
  } else {
    $publicManifest = $publicManifestText
  }
  foreach ($platformName in $platformArtifacts.Keys) {
    $platform = $publicManifest.platforms.PSObject.Properties[$platformName].Value
    $expectedUrl = "https://github.com/$Repository/releases/download/$Tag/$($platformArtifacts[$platformName])"
    if ($platform.url -ne $expectedUrl -or [string]::IsNullOrWhiteSpace($platform.signature)) {
      throw "Published updater entry for $platformName is incomplete or unstable."
    }
  }

  Write-Output "Verified and published $Tag with $($requiredAssets.Count) release assets."
} finally {
  Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force -ErrorAction SilentlyContinue
}
