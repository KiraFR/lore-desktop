# Installs the SoonerOrLater Studio code-signing certificate (public part) into
# the machine trust stores, so Windows and app-control tools accept binaries
# signed by the studio (Lore Desktop app + installer, auto-updates included).
#
# Run ONCE per machine, from an elevated (Administrator) PowerShell:
#   powershell -ExecutionPolicy Bypass -File scripts\trust-studio-cert.ps1
#
# This only imports a PUBLIC certificate — no private key is involved.

#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"
$cer = Join-Path $PSScriptRoot "..\certs\SoonerOrLater-CodeSigning.cer"
if (-not (Test-Path $cer)) { throw "Certificate not found: $cer" }

Import-Certificate -FilePath $cer -CertStoreLocation Cert:\LocalMachine\Root | Out-Null
Import-Certificate -FilePath $cer -CertStoreLocation Cert:\LocalMachine\TrustedPublisher | Out-Null

Write-Host "SoonerOrLater code-signing certificate trusted (Root + TrustedPublisher)."
