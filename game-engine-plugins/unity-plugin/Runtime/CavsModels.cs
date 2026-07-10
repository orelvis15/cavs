// Serializable request/result models for the CAVS SDK operations most
// relevant to a Unity runtime. Field names are camelCase to match the SDK's
// JSON envelope, so Unity's JsonUtility (de)serializes them directly.
//
// STATUS: untested.

using System;

namespace Cavs
{
    /// A progress event as delivered by the native progress callback.
    [Serializable]
    public class ProgressEvent
    {
        public string type;        // "started" | "phaseChanged" | "progress" | "completed" | "failed"
        public string operation;
        public string phase;
        public long currentBytes;
        public long totalBytes;
        public double percentage;  // 0..1
        public string message;
    }

    /// Install/update a build straight from a static export
    /// (`cavs store export --static-plans`) with no game server.
    [Serializable]
    public class FetchStaticRequest
    {
        /// Base URL or local directory of the static export.
        public string base_;      // serialized as "base" (see CavsClient)
        /// Asset name (the `<name>` under `assets/<name>/`).
        public string asset;
        /// Output directory for the reconstructed build.
        public string outputDir;
        /// Persistent content-addressable cache directory.
        public string cacheDir;
        /// Concurrent range requests (0 = default 8).
        public int connections;
        /// Optional Ed25519 public key (64 hex) enforcing the content signature.
        public string pubkey;
    }

    [Serializable]
    public class FetchStaticResult
    {
        public string asset;
        public string outputDir;
        public long wireBytes;
        public long rawBytes;
        public long chunksFetched;
        public long chunksReused;
        public long logicalBytes;
        public double savedPercent;
    }
}
