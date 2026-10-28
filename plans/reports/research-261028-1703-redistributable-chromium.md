# Redistributable Chromium candidate

Research: 2026-10-28, Asia/Saigon. Current primary sources and artifact receipts reviewed. This researcher fetched source text and small source archives into memory; did not download a browser, download the large Chromium source archive, or launch a browser.

## Decision

**Reject official snapshot 1654400 for redistribution. Select ungoogled Chromium Windows x64 156.0.8078.47-1.1 for integration and the next exact runtime packet.** The replacement passes the existing runner's checks and supplies full embedded credits. Redistribution remains conditional on verified corresponding source/build material delivered with the installer and review of the exact runtime packet. A source hyperlink alone does not close static FFmpeg obligations.

This preserves packaged Node, Playwright and a browser inside the Windows installer, using explicit `executablePath`. No first-launch browser download, installed-browser dependency or Electron change is required. This report changes no product configuration, human approval record or publication state.

Navigation: [rejected snapshot](#rejected-snapshot-1654400), [selected replacement](#selected-replacement), [source route](#source-delivery-and-static-ffmpeg), [descriptor](#machine-readable-descriptor-proposal), [remaining gates](#remaining-gates).

## Rejected snapshot 1654400

The earlier conditional selection is superseded by actual evidence. [Artifact report](qa-261028-1703-chromium-artifact.md).

| Field | Verified value |
|---|---|
| Binary | [Official Win_x64/1654400/chrome-win.zip](https://storage.googleapis.com/chromium-browser-snapshots/Win_x64/1654400/chrome-win.zip) |
| Chromium source/version | `59e897bb7c08867380065818882e6c7d2de599ef` / `151.0.7922.0` |
| Object generation/size | `1782775688693704` / `340981417` bytes |
| Publisher metadata | MD5 `c231fc11bd079d1b58314c81a41d590f`; CRC32C base64 `xjra7g==` |
| Downloaded archive SHA256 | `ea72622764ec197569ad418a92678430724211cd31cabea07dddeca50cbf74d9`, locally calculated |
| Extracted inventory | 261 files / 810425568 bytes |
| Runner | 8 actions and 9 assertions passed, including isolation, screenshot and close/abort cleanup |
| Credits | 177-byte placeholder text; zero component licenses; HTML 446 bytes |
| Other inventory | No loose licenses, Widevine or `ffmpeg.dll`; deliberately blank 93-byte terms page |

Receipts: archive (local QA evidence; not published), inventory (local QA evidence; not published), compatibility (local QA evidence; not published), notices (local QA evidence; not published). Publisher provenance: [Google receipt](https://www.googleapis.com/storage/v1/b/chromium-browser-snapshots/o/Win_x64%2F1654400%2Fchrome-win.zip), [REVISIONS](https://storage.googleapis.com/chromium-browser-snapshots/Win_x64/1654400/REVISIONS).

The exact [Chromium LICENSE](https://chromium.googlesource.com/chromium/src/+/59e897bb7c08867380065818882e6c7d2de599ef/LICENSE) permits binary redistribution of covered code subject to its conditions. It does not supply missing component notices. The [builder flags](https://chromium.googlesource.com/chromium/src/+/59e897bb7c08867380065818882e6c7d2de599ef/infra/config/generated/builders/ci/win-archive-rel/gn-args.json) and pinned [FFmpeg build](https://chromium.googlesource.com/chromium/third_party/ffmpeg/+/ad41607c61898cf7150e0fb20fe4bbabd44922a3/BUILD.gn) establish static integration into the linked browser. A shared-library LGPL route is unproven. Compatibility passed; notices and corresponding-source closure did not. This June snapshot also cannot be presented as a maintained current release.

## Selected replacement

| Field | Verified value |
|---|---|
| Publisher/release | [ungoogled-software Windows 156.0.8078.47-1.1](https://github.com/ungoogled-software/ungoogled-chromium-windows/releases/tag/156.0.8078.47-1.1), published `2026-10-25T16:46:25Z` |
| Archive | Exact URL, size and SHA256 in the descriptor below |
| Hash provenance | GitHub release API's published asset digest, independently matched by the spike; not a detached publisher signature |
| Platform/core commits | `258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af` / `0673f8701c94e5688c33ef8ce5e36602d311d246` |
| Chromium source commit | `1d17a2e2ee147bb7d6ae2ab6abdb3a2510c17775`, from [official version lookup](https://chromiumdash.appspot.com/fetch_version?version=156.0.8078.47) |
| Actual executable | Chromium `156.0.8078.47`; SHA256 `b211ae4d314dafae9692ffe2a0090b32fc58fe18fd8c841ad12d471615140200` |
| Actual extraction | 75 files / 444915353 bytes; no Widevine payload, standalone `ffmpeg.dll` or `dxil.dll` |
| Runner evidence | 8 actions and 9 assertions passed against the actual runner, including isolation, screenshot and close/abort cleanup |
| Full credits | 497 entries/497 license bodies; 5224021 text bytes / 5350295 HTML bytes |
| Credits text SHA256 | `9c48aad6914260957008db581aa676e1d39d584d15863b80d9fb17f621f644d5` |
| Credits HTML SHA256 | `db501f47d0d1edf716f1293b9ea12f3802929f005b3ed1c286e4e80909f0e1a9` |
| Actual terms | Deliberately blank 93-byte text; stays at `chrome://terms/`; no external redirect |

Receipts: archive (local QA evidence; not published), inventory (local QA evidence; not published), compatibility (local QA evidence; not published), component summary (local QA evidence; not published), full credits text (local QA evidence; not published), full credits HTML (local QA evidence; not published). [Pinned publisher API](https://api.github.com/repos/ungoogled-software/ungoogled-chromium-windows/releases/tags/156.0.8078.47-1.1).

The [platform flags](https://github.com/ungoogled-software/ungoogled-chromium-windows/blob/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af/flags.windows.gn) select `is_component_build=false`, `is_official_build=true`, `proprietary_codecs=true`, and `ffmpeg_branding="Chrome"`. That last flag selects codec configuration; it does not make this independently built browser Google's Chrome for Testing. The [core flags](https://github.com/ungoogled-software/ungoogled-chromium/blob/0673f8701c94e5688c33ef8ce5e36602d311d246/flags.gn) enable Widevine capability; the inventory contains no Widevine binary.

The credits include cross-platform entries: 497 is the retained notice count, not proof all 497 execute on Windows. Preserve full texts and Chromium's and both ungoogled projects' root licenses. Keep separately licensed DirectX compiler/SDK DLL notices; a repository BSD badge does not replace them. Actual `d3dcompiler_47.dll` SHA256 is `3e56c1a0867aa04cbca948b3664fcac5d51275865cd044a45a3fd6ce636ee914`; `dxcompiler.dll` is `ea756b8fb9fbfd44dcf29acf29e8d2b1a78c421de5c429d2615d9ac97c3f2143`. Pinned [Dawn metadata](https://dawn.googlesource.com/dawn/+/1da41e55bb915dd03f61028fad64a99ee1786d48/third_party/directx-shader-compiler/README.chromium) identifies DXC revision `9757d44faa47de7e8d1bb731c30551ec608624f9` and its mixed licenses. Microsoft's [SDK guidance](https://learn.microsoft.com/en-us/windows/win32/directx-sdk--august-2009-) permits D3DCompiler redistribution beside an application; retain applicable SDK terms.

## Source delivery and static FFmpeg

Exact [Chromium DEPS](https://chromium.googlesource.com/chromium/src/+/1d17a2e2ee147bb7d6ae2ab6abdb3a2510c17775/DEPS) pins FFmpeg `9db86ce5b5b454dfc96c435f9f0723012980c37f`. Its [metadata](https://chromium.googlesource.com/chromium/third_party/ffmpeg/+/9db86ce5b5b454dfc96c435f9f0723012980c37f/README.chromium) and [credits](https://chromium.googlesource.com/chromium/third_party/ffmpeg/+/9db86ce5b5b454dfc96c435f9f0723012980c37f/CREDITS.chromium) identify shipped LGPL-2.1-or-later code plus permissive terms. Its selected [Chrome/win/x64 configuration](https://chromium.googlesource.com/chromium/third_party/ffmpeg/+/9db86ce5b5b454dfc96c435f9f0723012980c37f/chromium/config/Chrome/win/x64/config.h) sets `CONFIG_GPL=0`, `CONFIG_NONFREE=0`, `CONFIG_VERSION3=0`. Optional GPL text in credits does not establish a GPL-configured binary.

Use the [LGPL 2.1 section 6(a) material route with section 6(d) equivalent access](https://chromium.googlesource.com/chromium/third_party/ffmpeg/+/9db86ce5b5b454dfc96c435f9f0723012980c37f/COPYING.LGPLv2.1): provide complete library source and sufficient linked-work source/object material for modification and relinking. FFmpeg source alone is insufficient. Preserve notices/license and permit own-use modification and debugging reverse engineering. Deliver materials at the binary's download place. Do not substitute a written source offer, generic homepage or user-installed FFmpeg DLL.

### Source input evidence

The pinned core [downloads.ini](https://github.com/ungoogled-software/ungoogled-chromium/blob/0673f8701c94e5688c33ef8ce5e36602d311d246/downloads.ini) explicitly chooses the official lite tarball. Its [publisher hash file](https://storage.googleapis.com/chromium-browser-official/chromium-156.0.8078.47-lite.tar.xz.hashes) supplies the descriptor's SHA256. Google Storage metadata records size `2009127748`, generation `1792539541153162`, MD5 base64 `3NDX7EVf3xjkWRyo72+dQw==`, CRC32C base64 `wjM/Og==`.

The reviewed official [lite publishing recipe](https://chromium.googlesource.com/chromium/tools/build/+/8a0253c87d601932e6a80c57faa0f6b5214921ae/recipes/recipes/publish_tarball.py) prunes other-platform material, tests and toolchains; it does not prune FFmpeg or desktop browser source. Its [exporter](https://chromium.googlesource.com/chromium/tools/build/+/8a0253c87d601932e6a80c57faa0f6b5214921ae/recipes/recipe_modules/chromium/resources/export_tarball.py) retains GN/build metadata. The [platform build script](https://github.com/ungoogled-software/ungoogled-chromium-windows/blob/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af/build.py) supports `--tarball`. The [Windows download manifest](https://github.com/ungoogled-software/ungoogled-chromium-windows/blob/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af/downloads.ini) identifies four target-source replacements, all included below. These provide a concrete source/build route; the actual 2 GB archive remains a hosted validation gate.

### Hosted packet and member gate

Download all seven source archives; verify whole-file bytes and SHA256 before inspecting members. List without extraction with `tar -tJf`, `tar -tzf` or Python `tarfile` streaming mode. Write a descriptor-bound receipt of hashes and member matches. Preserve every source, patch, generated input and configuration in the archives. Minimum member samples do not prove a successful rebuild or complete closure alone.

Under `chromium-156.0.8078.47/`, require:

```text
LICENSE
DEPS
BUILD.gn
chrome/VERSION
build/config/BUILDCONFIG.gn
third_party/ffmpeg/BUILD.gn
third_party/ffmpeg/ffmpeg_options.gni
third_party/ffmpeg/COPYING.LGPLv2.1
third_party/ffmpeg/CREDITS.chromium
third_party/ffmpeg/libavcodec/avcodec.h
third_party/ffmpeg/libavutil/avutil.h
third_party/ffmpeg/chromium/config/Chrome/win/x64/config.h
```

Platform archive: `build.py`, `package.py`, `flags.windows.gn`, `downloads.ini`, `patches/series`, `LICENSE`. Core archive: `flags.gn`, `downloads.ini`, `utils/patches.py`, `patches/series`, `pruning.list`, `domain_regex.list`, `LICENSE`. Validate supplemental archives' headers/crate sources and licenses too.

Deliver all seven source archives, complete notices, descriptor, validation receipt and build/relink instructions **inside the same downloadable Actions artifact as the installer**. An Actions artifact may expire only when its installer becomes unavailable too. A later durable public installer needs durable source co-delivery from the same release download place. This route requires no owner written source offer.

### Build and relink route

Use a short Windows path. Platform archive root: `ungoogled-chromium-windows-258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af`. Core root: `ungoogled-chromium-0673f8701c94e5688c33ef8ce5e36602d311d246`. Place core root **contents** into platform `ungoogled-chromium/`; the platform archive omits submodule contents. Put delivered source inputs into `build/download_cache/` with descriptor filenames.

Prerequisites: Visual Studio C++ tools, Windows SDK `10.0.28000.0`, Python `3.12`, 7-Zip, `httplib2==0.22.0`, `clang==21.1.7`. Manifest pins LLVM `23.1.0`, Rust nightly `2026-09-09`, Ninja and remaining tools. [CI preparation](https://github.com/ungoogled-software/ungoogled-chromium-windows/blob/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af/.github/actions/prepare/action.yml).

In a VS x64 developer shell at the platform root:

```powershell
python -m pip install httplib2==0.22.0 clang==21.1.7
python build.py --tarball
python package.py
```

The script applies core then Windows patches, substitutes domains and combines flag files into `build/src/out/Default/args.gn`. Tarball mode appends `chrome_pgo_phase=0`; original [release CI](https://github.com/ungoogled-software/ungoogled-chromium-windows/blob/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af/.github/actions/stage/index.js) uses clone mode and phase 2. State this optimization difference; do not promise byte-identical reproduction.

After initial build, edit `build/src/third_party/ffmpeg` and relink:

```powershell
$env:DEPOT_TOOLS_WIN_TOOLCHAIN = '0'
Set-Location build/src
third_party\ninja\ninja.exe -C out\Default chrome chromedriver mini_installer
Set-Location ../..
python package.py
```

Do not rerun non-CI preparation after edits: unpacking can overwrite modifications. Ninja preserves them. Outputs: `build/src/out/Default`; packages: `build/`. Commands are source-verified, not executed here. Toolchain downloads are build prerequisites; browser end users need none of these downloads.

## Machine-readable descriptor proposal

The controller owns [release/runner-browser-source.json](../../release/runner-browser-source.json). Existing binary and three initial source pins match this research. Complete seven-input proposal below. GitHub source archive SHA256 values were locally computed, not publisher signatures. The windows-rs SHA512 also matches the pinned Windows manifest.

```json
{
  "schemaVersion": 1,
  "name": "ungoogled Chromium for Windows x64",
  "version": "156.0.8078.47-1.1",
  "browserVersion": "156.0.8078.47",
  "chromiumSourceCommit": "1d17a2e2ee147bb7d6ae2ab6abdb3a2510c17775",
  "platformCommit": "258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af",
  "coreCommit": "0673f8701c94e5688c33ef8ce5e36602d311d246",
  "archiveRoot": "ungoogled-chromium_156.0.8078.47-1.1_windows_x64",
  "browserExecutable": "browsers/chromium-156.0.8078.47-1.1/chrome.exe",
  "archive": {
    "url": "https://github.com/ungoogled-software/ungoogled-chromium-windows/releases/download/156.0.8078.47-1.1/ungoogled-chromium_156.0.8078.47-1.1_windows_x64.zip",
    "bytes": 201848261,
    "sha256": "e8ed1486a88146bf72bfe3d22e198808560e16803a9a1ef62e6e3fc17c979800"
  },
  "sourceMaterials": [
    {
      "name": "chromium-156.0.8078.47-lite.tar.xz",
      "url": "https://storage.googleapis.com/chromium-browser-official/chromium-156.0.8078.47-lite.tar.xz?generation=1792539541153162",
      "bytes": 2009127748,
      "sha256": "28432028890ef70e1f2e9db880c500907ad1b04f5ae6b655a3f394b9f2eaa705"
    },
    {
      "name": "ungoogled-chromium-windows-258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af.tar.gz",
      "url": "https://codeload.github.com/ungoogled-software/ungoogled-chromium-windows/tar.gz/258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af",
      "bytes": 415624,
      "sha256": "1cdfcea2b5d914ef567302cb814927759b47b10d1893d2432313797591d93ddc"
    },
    {
      "name": "ungoogled-chromium-0673f8701c94e5688c33ef8ce5e36602d311d246.tar.gz",
      "url": "https://codeload.github.com/ungoogled-software/ungoogled-chromium/tar.gz/0673f8701c94e5688c33ef8ce5e36602d311d246",
      "bytes": 679725,
      "sha256": "dd554b3d95abd7a2c55f9d78a37f3d2e9b59692d50ed016cb959d2ff148ce945"
    },
    {
      "name": "directx-headers-07b94cba474be6cdfd934febdbd0b21ca0524187.zip",
      "url": "https://github.com/microsoft/DirectX-Headers/archive/07b94cba474be6cdfd934febdbd0b21ca0524187.zip",
      "bytes": 551235,
      "sha256": "0545f43a34ac447770cc988d8631de41e6987fc02cc3d2ae82d7fba979f5398d"
    },
    {
      "name": "webauthn-ef82c157125a0490e05f6ea82a7adb1b8e1bad08.zip",
      "url": "https://github.com/microsoft/webauthn/archive/ef82c157125a0490e05f6ea82a7adb1b8e1bad08.zip",
      "bytes": 22947,
      "sha256": "f7a9c3c4fa7e6f557e8182d68db67cd9dad87d9f42fdb5adc39d0f2968e903ec"
    },
    {
      "name": "multimedia-redirection-8b1d8893d126547309f1bab8d8232418aa7c7109.zip",
      "url": "https://github.com/microsoft/multimedia-redirection/archive/8b1d8893d126547309f1bab8d8232418aa7c7109.zip",
      "bytes": 39178,
      "sha256": "d0451ade64f4a24ffbc0128683015f3e3bf40f02c90a4c4acef664e26088980f"
    },
    {
      "name": "rust-windows-create-0.52.0.zip",
      "url": "https://github.com/microsoft/windows-rs/archive/refs/tags/0.52.0.zip",
      "bytes": 35516630,
      "sha256": "96379ad8ab18901040f007a0fe27431edd7b16db5e3853d90f0f4ee9be5aa926"
    }
  ],
  "sourceDelivery": "Deliver verified sourceMaterials, full notices and build/relink instructions in the same downloadable artifact as the installer. Do not offer the installer when its source packet is unavailable. No written source offer is substituted."
}
```

## Remaining gates

- **Closed:** snapshot rejection; replacement binary identity/pin; actual runner compatibility; full embedded credits capture; practical source route; exact platform/core/target-source inputs.
- **Hosted gate:** download/hash-check the lite archive, verify all seven archive contents, preserve source/build materials and bind the receipt to the installer packet. No local source download/extraction is needed.
- **Runtime packet gate:** combine Node, Playwright and browser notices, source route and hashes into one exact packet. Human review is required before changing its approval record. Preserve modification/debugging permissions in applicable terms.
- **Distribution gate:** keep installer and corresponding source available together. This research does not grant patent rights beyond component terms or approve public branding/publication.

**Conditional GO for implementing this packaging route. NO-GO for claiming completed redistribution clearance before the hosted source receipt and exact runtime packet review.** No written-source-offer commitment is required by the selected route.

## Unresolved questions

- Does the hosted packet's actual source/member receipt pass every required gate?
- Has a human approved the exact new Node/Playwright/Chromium notice and source-delivery packet?
