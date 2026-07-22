# Publishing a release

1. Bump `version` in `src-tauri/tauri.conf.json` (source of truth; `package.json` stays at 0.0.0) and commit.
2. Tag and push: `git tag -a vX.Y.Z -m "notes"` then `git push origin vX.Y.Z` — the tag message becomes the release notes.
3. `.github/workflows/release.yml` builds the signed NSIS setup on `windows-latest`, generates `latest.json` via `scripts/make-latest-json.mjs` (tested by `node scripts/make-latest-json.test.mjs`), and publishes everything as a GitHub release; the CI fails if the tag and `tauri.conf.json` versions differ.

Required repo secrets (set by Jimmy only): `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (minisign key from `npx tauri signer generate`), plus `WINDOWS_CERT_PFX_BASE64` and `WINDOWS_CERT_PASSWORD` (Authenticode, below).

## Code signing (Authenticode, studio-internal)

Binaries and the NSIS setup are Authenticode-signed during bundling with the studio's self-signed certificate (`certs/SoonerOrLater-CodeSigning.cer`, thumbprint `7820BD9C842F7D9A0A91917967476AACCBFFE3E0`, expires 2031-07-22) so app-control tools accept the auto-update installer launched from temp. Each studio machine trusts it ONCE by running `scripts/trust-studio-cert.ps1` from an elevated PowerShell (public cert only — imported into LocalMachine Root + TrustedPublisher). Signing config lives in `tauri.conf.json` (`bundle.windows.certificateThumbprint`); local builds sign too when the certificate is in the builder's user store.
