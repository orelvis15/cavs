# CAVS plugin for Unity

> **Status: UNTESTED.** This package compiles against the CAVS SDK C ABI
> (`cavs_sdk.h`, ABI 1.0.0) and mirrors the shipping Kotlin (FFM) and Node
> (koffi) bindings 1:1, but it has **not yet been validated inside the Unity
> editor or on a device**. Treat it as a reference integration, not a
> supported release. Feedback and fixes are welcome.

A Unity package that brings CAVS content delivery to AssetBundles and
Addressables: install and update builds by downloading only the chunks that
changed between versions — straight from a **static CDN export**, with no game
server — then verify and use the reconstructed files at runtime.

It wraps the same compiled Rust core the CLI and other SDKs use, through the
stable C ABI (`libcavs_sdk`). No CVSP re-implementation, no shelling out.

## Install

1. Copy this folder into your project's `Packages/` directory (or add it via
   the Package Manager "add from disk").
2. Download the `cavs-sdk-native-<version>-<target>` artifact for each
   platform you ship from the project's releases and place the native library
   under `Plugins/` so Unity bundles it:

   | Platform | File | Unity Plugins path |
   |---|---|---|
   | Windows x64 | `cavs_sdk.dll` | `Plugins/x86_64/` |
   | macOS (universal) | `libcavs_sdk.dylib` | `Plugins/macOS/` |
   | Linux x64 | `libcavs_sdk.so` | `Plugins/x86_64/` |
   | Android arm64 | `libcavs_sdk.so` | `Plugins/Android/arm64-v8a/` |
   | iOS arm64 | `libcavs_sdk.a` (static) | `Plugins/iOS/` |

   `DllImport` uses the base name `cavs_sdk`; Unity resolves the platform
   prefix/suffix. (Native artifacts currently ship for desktop targets; mobile
   targets require building `cavs-ffi` for those toolchains — see
   `docs/SDK_NATIVE_ABI.md`.)

## Use

```csharp
using Cavs;

using var client = new CavsClient();

var result = await client.FetchStaticAsync(new FetchStaticRequest {
    base_      = "https://cdn.example.com/game",  // a `store export --static-plans` tree
    asset      = "game",
    outputDir  = Path.Combine(Application.persistentDataPath, "cavs/install"),
    cacheDir   = Path.Combine(Application.persistentDataPath, "cavs/cache"),
    connections = 8,
}, onProgress: ev => Debug.Log($"{ev.percentage:P0}"));

Debug.Log($"fetched {result.chunksFetched}, reused {result.chunksReused}, " +
          $"saved {result.savedPercent:F1}%");
```

The first install downloads the whole build once; every later launch downloads
only the chunks that changed, reusing the persistent cache. Reconstruction is
byte-identical and BLAKE3-verified or it fails — a torn file is never promoted.

A ready-to-drop sample lives in `Samples~/SelfUpdate/`.

## Publishing content for this plugin

On your build machine (see the repo README for details):

```sh
# Package the build, ingest into a packfile store, export a static tree
cavs pack-dir ./Build --profile auto -o build.cavs
cavs store ./store add game build.cavs --storage packfiles
cavs store ./store export --out ./dist --static-plans
# Upload ./dist to S3 / R2 / GitHub Pages / any static host.
```

The client only needs the exported `dist/` tree; no `cavs-server` runs in
production.

## What works vs. what's planned

- **Implemented (untested):** `FetchStaticAsync` (serverless install/update
  with progress + cancellation), `Execute(op, json)` for any SDK operation,
  version/capabilities.
- **Planned:** an editor post-process that packages AssetBundle/Addressables
  output into a `.cavs` + static export in one click; typed wrappers for the
  analyze/preview/pack operations.
