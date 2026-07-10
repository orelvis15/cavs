# CAVS plugin for Unreal Engine

> **Status: UNTESTED.** This module compiles against the CAVS SDK C ABI
> (`cavs_sdk.h`, ABI 1.0.0, vendored under
> `Source/ThirdParty/CavsSdkLibrary/include`) and mirrors the shipping SDK
> bindings, but it has **not yet been built with UnrealBuildTool or run
> in-editor**. Treat it as a reference integration, not a supported release.

An Unreal plugin that brings CAVS content delivery to PAK / IoStore
(`.ucas` / `.utoc`) containers and cooked content: install and update builds
by downloading only the chunks that changed between versions — straight from a
**static CDN export**, with no game server — verified and reconstructed on the
client. It wraps the same compiled Rust core the CLI and other SDKs use,
through the stable C ABI (`libcavs_sdk`).

## Install

1. Copy this folder into your project's `Plugins/` directory (as
   `Plugins/CavsSdk/`).
2. Download the `cavs-sdk-native-<version>-<target>` artifact for each
   platform you ship and place the library under
   `Source/ThirdParty/CavsSdkLibrary/lib/<Platform>/`:

   | Platform | Files |
   |---|---|
   | `Win64` | `cavs_sdk.dll`, `cavs_sdk.dll.lib` |
   | `Mac` | `libcavs_sdk.dylib` |
   | `Linux` | `libcavs_sdk.so` |

3. Regenerate project files and build. Enable the **CAVS Content Delivery**
   plugin in the editor.

## Use (Blueprint or C++)

```cpp
UCavsClient* Client = NewObject<UCavsClient>();
Client->OnProgress.AddDynamic(this, &AMyActor::HandleProgress);
Client->OnCompleted.AddDynamic(this, &AMyActor::HandleCompleted);
Client->OnFailed.AddDynamic(this, &AMyActor::HandleFailed);

FCavsFetchStaticRequest Req;
Req.Base      = TEXT("https://cdn.example.com/game");  // a `store export --static-plans` tree
Req.Asset     = TEXT("game");
Req.OutputDir = FPaths::Combine(FPaths::ProjectPersistentDownloadDir(), TEXT("cavs/install"));
Req.CacheDir  = FPaths::Combine(FPaths::ProjectPersistentDownloadDir(), TEXT("cavs/cache"));
Req.Connections = 8;
Client->FetchStatic(Req);
```

`FetchStatic` runs on a background thread; `OnProgress` / `OnCompleted` /
`OnFailed` fire on the game thread. The first install downloads the whole
build once; later launches download only the chunks that changed, reusing the
persistent cache. Reconstruction is byte-identical and BLAKE3-verified or it
fails.

## Publishing content

```sh
cavs pack-dir ./Cooked --profile auto -o build.cavs
cavs store ./store add game build.cavs --storage packfiles
cavs store ./store export --out ./dist --static-plans
# Upload ./dist to any static host; no cavs-server runs in production.
```

Unreal-specific packaging tips (avoiding IoStore offset-cascade update bloat)
come from `cavs analyze steampipe` and `cavs analyze-packs` — see
`docs/BUILD_UPDATE_ANALYZER.md`.

## What works vs. planned

- **Implemented (untested):** `UCavsClient::FetchStatic` (serverless
  install/update with progress, completion and cancellation delegates),
  `Version()`.
- **Planned:** a cook-time hook that packages PAK/IoStore output into a `.cavs`
  + static export; Blueprint nodes for analyze/preview.
