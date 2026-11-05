# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD
#
# Runtime verification of `pasejo browser install-manifest` on Windows: it
# runs the command, then asserts the three artifacts the registration must
# produce — the HKCU registry value, the manifest JSON it points at, and the
# `.cmd` wrapper that launches `pasejo browser host`. This is the part of the
# Windows path that can be checked without a real browser; it runs in CI on a
# `windows-latest` runner and can be run by hand from a Windows checkout.
#
# Uses a throwaway `--host-name` so it never touches a real pasejo native-
# messaging registration, and cleans up after itself.

$ErrorActionPreference = 'Stop'

$HostName = 'wtf.metio.pasejo.citest'
# A chromium extension id is 32 chars in a-p; the value is never validated, it
# only has to land in the manifest's allowed_origins.
$ExtensionId = 'abcdefghijklmnopabcdefghijklmnop'
$RegistryKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\$HostName"

function Cleanup {
    if (Test-Path -LiteralPath $RegistryKey) {
        $manifest = (Get-Item -LiteralPath $RegistryKey).GetValue('')
        if ($manifest -and (Test-Path -LiteralPath $manifest)) {
            $wrapper = (Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json).path
            if ($wrapper -and (Test-Path -LiteralPath $wrapper)) {
                Remove-Item -LiteralPath $wrapper -Force -ErrorAction SilentlyContinue
            }
            Remove-Item -LiteralPath $manifest -Force -ErrorAction SilentlyContinue
        }
        Remove-Item -LiteralPath $RegistryKey -Force -ErrorAction SilentlyContinue
    }
}

try {
    Cleanup  # start from a clean slate even if a prior run was interrupted

    Write-Host '==> running: pasejo browser install-manifest --browser chrome'
    cargo run --quiet -- browser install-manifest `
        --browser chrome --extension-id $ExtensionId --host-name $HostName
    if ($LASTEXITCODE -ne 0) {
        throw "install-manifest exited with code $LASTEXITCODE"
    }

    # 1. The registry value exists and names a manifest file.
    if (-not (Test-Path -LiteralPath $RegistryKey)) {
        throw "registry key not created: $RegistryKey"
    }
    $manifestPath = (Get-Item -LiteralPath $RegistryKey).GetValue('')
    if (-not $manifestPath) {
        throw "registry key $RegistryKey has no default value"
    }
    Write-Host "[ok] registry $RegistryKey -> $manifestPath"

    # 2. The manifest exists and has the expected native-messaging shape.
    if (-not (Test-Path -LiteralPath $manifestPath)) {
        throw "manifest file missing: $manifestPath"
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.name -ne $HostName) {
        throw "manifest name is '$($manifest.name)', expected '$HostName'"
    }
    if ($manifest.type -ne 'stdio') {
        throw "manifest type is '$($manifest.type)', expected 'stdio'"
    }
    if (-not ($manifest.allowed_origins -contains "chrome-extension://$ExtensionId/")) {
        throw "manifest allowed_origins missing chrome-extension://$ExtensionId/"
    }
    if (-not $manifest.path.EndsWith('.cmd')) {
        throw "manifest path is not a .cmd wrapper: $($manifest.path)"
    }
    Write-Host "[ok] manifest shape (name/type/allowed_origins/.cmd path)"

    # 3. The .cmd wrapper exists and launches `browser host`.
    $wrapperPath = $manifest.path
    if (-not (Test-Path -LiteralPath $wrapperPath)) {
        throw ".cmd wrapper missing: $wrapperPath"
    }
    if (-not (Select-String -LiteralPath $wrapperPath -Pattern 'browser host' -Quiet)) {
        throw "wrapper $wrapperPath does not invoke 'browser host'"
    }
    Write-Host "[ok] .cmd wrapper invokes 'browser host'"

    Write-Host '==> PASS: native-messaging host registered (registry + manifest + .cmd wrapper)'
}
finally {
    Cleanup
}
