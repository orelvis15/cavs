// Sample: self-update a build from a static CAVS export at startup, showing
// progress, then load an AssetBundle from the reconstructed output.
//
// STATUS: untested sample — illustrates the intended flow.

using System;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace Cavs.Samples
{
    public class CavsUpdater : MonoBehaviour
    {
        [Tooltip("Base URL of the static export, e.g. https://cdn.example.com/game")]
        public string baseUrl = "https://cdn.example.com/game";

        [Tooltip("Asset name under assets/<name>/ in the export")]
        public string asset = "game";

        [Range(1, 32)] public int connections = 8;

        private CancellationTokenSource _cts;

        private async void Start()
        {
            _cts = new CancellationTokenSource();
            string output = System.IO.Path.Combine(Application.persistentDataPath, "cavs", "install");
            string cache = System.IO.Path.Combine(Application.persistentDataPath, "cavs", "cache");

            using (var client = new CavsClient())
            {
                Debug.Log($"CAVS SDK {CavsClient.Version} (ABI {CavsClient.AbiVersion})");
                var req = new FetchStaticRequest
                {
                    base_ = baseUrl,
                    asset = asset,
                    outputDir = output,
                    cacheDir = cache,
                    connections = connections,
                };
                try
                {
                    var result = await client.FetchStaticAsync(
                        req,
                        onProgress: ev =>
                        {
                            if (ev.type == "progress")
                                Debug.Log($"CAVS update {ev.percentage:P0} ({ev.currentBytes}/{ev.totalBytes} B)");
                        },
                        cancel: _cts.Token);

                    Debug.Log(
                        $"CAVS update complete: {result.chunksFetched} chunks fetched, " +
                        $"{result.chunksReused} reused, saved {result.savedPercent:F1}% of egress.");

                    // The reconstructed files are under `output`; load a bundle:
                    // var bundle = AssetBundle.LoadFromFile(
                    //     System.IO.Path.Combine(output, "game.assetbundle"));
                }
                catch (CavsException e)
                {
                    Debug.LogError($"CAVS update failed [{e.Code}]: {e.Message}");
                }
            }
        }

        private void OnDestroy() => _cts?.Cancel();
    }
}
