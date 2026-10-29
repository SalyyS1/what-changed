# Packaged browser sources and library modification

**Evidence state: source-delivery procedure; hosted archive checks and complete installer pending.**

The packaged preview browser is ungoogled Chromium for Windows x64. Its exact
binary, Chromium version, platform/core commits and source archives are pinned in
[the browser source descriptor](../release/runner-browser-source.json). The runtime
inventory separately records every installed file. Node and Ownvero execute as
separate programs; the browser's statically linked FFmpeg is the reason this
browser source and relinking route accompanies the installer.

The browser archive's `d3dcompiler_47.dll` is deliberately excluded from the
installer. Chromium's Windows loader falls back to the inbox
`%SystemRoot%\System32\D3DCompiler_47.dll`; the SDK side-copy carries separate
Microsoft redistribution terms. The supported OS prerequisite is Windows 10 or
11 x64 with that system component present. The package never downloads or copies
it. A clean-host acceptance check must still prove this fallback on both Windows
versions before treating the preview as broadly installable.

## Download layout

The hosted workflow must deliver the installer and `runtime-sources/` inside the
**same Actions artifact**, with the same access and retention. That directory
contains all descriptor-listed source archives, the exact descriptor, checksums
and a verification receipt. These are approximately 2 GB of source material,
separate from the approximately 550 MB installed runner. They are not downloaded
at application startup. Extract the outer Actions ZIP to obtain both the installer
and sources. The installer does not install the source archives on the PC.

Source preparation verifies each archive's exact size/hash and required source
members before installer upload. Missing sources stop the job. A download
containing only the installer is an incomplete redistribution packet. Keep the
corresponding sources available wherever that binary remains available. A later
release route must preserve equivalent source access; a link to an unrelated
upstream homepage or a written source offer is not substituted here.

## Rebuild the browser

Use a short Windows path with substantial free space; building Chromium requires
much more disk and RAM than running the app. Verify the delivered checksums first.
The frozen platform archive's README, `build.py`, `downloads.ini` and GN flags
own the precise prerequisites and commands.

1. Extract the platform source archive to a short working directory. Extract the
   core source archive **into its `ungoogled-chromium` subdirectory**: GitHub's
   platform archive does not contain its Git submodule's contents.
2. Install the platform build's public Visual Studio C++ tools, Windows SDK
   `10.0.28000.0`, Python 3.12 and 7-Zip. Install Python dependencies with
   `python -m pip install httplib2==0.22.0 clang==21.1.7`. See the pinned platform
   instructions for toolchain setup. Proprietary SDK/compiler binaries are not
   represented as Chromium source or relicensed by this project.
3. Copy the delivered Chromium lite archive and dependency override archives to
   `build/download_cache/` using the cache names in `downloads.ini`. The build
   scripts verify their upstream hashes and fetch required public build tools.
4. From the platform source root, run `python build.py --tarball`, then
   `python package.py`.

The preparation script unpacks Chromium, substitutes dependencies, applies core
then Windows patches, rewrites domains and combines `flags.gn` with
`flags.windows.gn` into `build/src/out/Default/args.gn`. It retains the exact FFmpeg
source and Windows x64 configuration used by that build family. Tarball mode
sets `chrome_pgo_phase=0`; upstream release CI used clone mode and PGO phase 2.
This is a source rebuild and relink procedure, **not byte-for-byte reproduction
of the upstream optimized executable**.

## Modify FFmpeg and relink

After initial preparation/build, edit the library under
`build/src/third_party/ffmpeg`. Open the Visual Studio x64 developer shell, then:

```powershell
$env:DEPOT_TOOLS_WIN_TOOLCHAIN = '0'
Set-Location build/src
third_party\ninja\ninja.exe -C out\Default chrome chromedriver mini_installer
Set-Location ../..
python package.py
```

Do not rerun the non-CI preparation command over these edits: it can unpack and
patch the source tree again. Preserve your modified source and build configuration.
The full FFmpeg license and other component terms are retained in the runtime
notices. Modification of LGPL libraries, and reverse engineering needed to debug
such modifications, are permitted by their applicable licenses; this project adds
no prohibition on those activities.

Ownvero verifies its packaged runtime inventory before launch. When substituting
your rebuilt browser, regenerate the local inventory for the changed files using
your own source build. Original review hashes will no longer describe those bytes;
do not label that modified runtime as the verified upstream binary. The source
inventory mechanism does not use a secret signing key.

## Scope of the evidence

The frozen build scripts, source archive members, complete embedded credits and
actual binary checks own the correspondence evidence. The supported source route
does not claim a reproducible browser build, successful installed acceptance,
network confinement or public-v1 approval. See
[development preview packaging](development-preview.md) for those open checks.
