// A managed wrapper over the CAVS SDK C ABI. One CavsClient owns one native
// context; reuse it across calls. The high-value runtime operation is
// FetchStatic — install/update a build from a static CDN export with no game
// server — but any SDK operation is reachable via Execute(op, requestJson).
//
// Threading: FetchStatic runs on a background native thread and is exposed as
// a coroutine-friendly polling handle plus a Task. Progress callbacks may
// arrive on the native worker thread; they are marshalled and surfaced via a
// thread-safe queue that the polling handle drains on the caller's thread.
//
// STATUS: untested. This compiles against the C ABI and mirrors the Kotlin
// FFM / Node koffi bindings, but has not yet been validated on a device.

using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace Cavs
{
    public sealed class CavsClient : IDisposable
    {
        private IntPtr _ctx;
        private NativeMethods.ProgressCallback _cbDelegate; // kept alive against GC
        private readonly object _lock = new object();
        private bool _disposed;

        // Progress events delivered from the native worker thread.
        private readonly ConcurrentQueue<ProgressEvent> _progress = new ConcurrentQueue<ProgressEvent>();

        public CavsClient()
        {
            _ctx = NativeMethods.cavs_context_new("{}");
            if (_ctx == IntPtr.Zero)
                throw new CavsException("CAVS-E-INTERNAL", "failed to create native context");

            _cbDelegate = OnNativeProgress;
            NativeMethods.cavs_context_set_progress_callback(_ctx, _cbDelegate, IntPtr.Zero);
        }

        public static string Version => NativeMethods.Utf8(NativeMethods.cavs_sdk_version());
        public static string AbiVersion => NativeMethods.Utf8(NativeMethods.cavs_sdk_abi_version());

        public static string Capabilities()
        {
            IntPtr p = NativeMethods.cavs_sdk_capabilities_json();
            try { return NativeMethods.Utf8(p); }
            finally { if (p != IntPtr.Zero) NativeMethods.cavs_string_free(p); }
        }

        private void OnNativeProgress(IntPtr eventJson, IntPtr userData)
        {
            var json = NativeMethods.Utf8(eventJson);
            if (json == null) return;
            try { _progress.Enqueue(JsonUtility.FromJson<ProgressEvent>(json)); }
            catch { /* ignore malformed progress */ }
        }

        /// Execute an operation synchronously, returning the `data` JSON on
        /// success or throwing CavsException on failure.
        public string Execute(string operation, string requestJson)
        {
            lock (_lock)
            {
                ThrowIfDisposed();
                string body = "{\"schemaVersion\":\"1.0\",\"data\":" + requestJson + "}";
                IntPtr result = NativeMethods.cavs_execute_json(_ctx, operation, body);
                return DrainResult(result);
            }
        }

        /// Install or update a build from a static export, off the main thread.
        /// `onProgress` (optional) is invoked on the calling task's thread as
        /// events are drained. Honours `cancel`.
        public Task<FetchStaticResult> FetchStaticAsync(
            FetchStaticRequest req,
            Action<ProgressEvent> onProgress = null,
            CancellationToken cancel = default)
        {
            string requestJson = BuildFetchStaticJson(req);
            return Task.Run(() =>
            {
                IntPtr job;
                lock (_lock)
                {
                    ThrowIfDisposed();
                    string body = "{\"schemaVersion\":\"1.0\",\"data\":" + requestJson + "}";
                    job = NativeMethods.cavs_start_json(_ctx, "fetchStatic", body);
                }
                if (job == IntPtr.Zero)
                    throw new CavsException("CAVS-E-INTERNAL", "failed to start fetchStatic");

                try
                {
                    IntPtr result;
                    while ((result = NativeMethods.cavs_job_poll(job)) == IntPtr.Zero)
                    {
                        if (cancel.IsCancellationRequested)
                            NativeMethods.cavs_job_cancel(job);
                        DrainProgress(onProgress);
                        Thread.Sleep(15);
                    }
                    DrainProgress(onProgress);
                    string data = DrainResult(result);
                    return JsonUtility.FromJson<FetchStaticResult>(data);
                }
                finally
                {
                    NativeMethods.cavs_job_free(job);
                }
            }, cancel);
        }

        private void DrainProgress(Action<ProgressEvent> onProgress)
        {
            while (_progress.TryDequeue(out var ev))
                onProgress?.Invoke(ev);
        }

        // JsonUtility cannot emit a field literally named "base" (a keyword),
        // so build the fetchStatic request object by hand.
        private static string BuildFetchStaticJson(FetchStaticRequest r)
        {
            string J(string s) => "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
            var sb = new System.Text.StringBuilder();
            sb.Append("{");
            sb.Append("\"base\":").Append(J(r.base_)).Append(",");
            sb.Append("\"asset\":").Append(J(r.asset)).Append(",");
            sb.Append("\"outputDir\":").Append(J(r.outputDir)).Append(",");
            sb.Append("\"cacheDir\":").Append(J(r.cacheDir));
            if (r.connections > 0) sb.Append(",\"connections\":").Append(r.connections);
            if (!string.IsNullOrEmpty(r.pubkey)) sb.Append(",\"pubkey\":").Append(J(r.pubkey));
            sb.Append("}");
            return sb.ToString();
        }

        /// Extract the `data` JSON from a CavsResult, or throw and always free.
        private string DrainResult(IntPtr result)
        {
            if (result == IntPtr.Zero)
                throw new CavsException("CAVS-E-INTERNAL", "null result");
            try
            {
                bool ok = NativeMethods.cavs_result_ok(result) == 1;
                string json = NativeMethods.Utf8(NativeMethods.cavs_result_json(result));
                if (!ok)
                {
                    string code = NativeMethods.Utf8(NativeMethods.cavs_result_error_code(result)) ?? "CAVS-E-INTERNAL";
                    string msg = NativeMethods.Utf8(NativeMethods.cavs_result_error_message(result)) ?? "operation failed";
                    throw new CavsException(code, msg);
                }
                // The envelope is {ok, operation, data:{...}}; return the data object.
                int idx = json.IndexOf("\"data\":", StringComparison.Ordinal);
                return idx >= 0 ? ExtractObject(json, idx + 7) : json;
            }
            finally
            {
                NativeMethods.cavs_result_free(result);
            }
        }

        // Extract the balanced JSON object/array starting at `start`.
        private static string ExtractObject(string json, int start)
        {
            while (start < json.Length && json[start] != '{' && json[start] != '[') start++;
            if (start >= json.Length) return "{}";
            char open = json[start];
            char close = open == '{' ? '}' : ']';
            int depth = 0; bool inStr = false; bool esc = false;
            for (int i = start; i < json.Length; i++)
            {
                char c = json[i];
                if (inStr) { if (esc) esc = false; else if (c == '\\') esc = true; else if (c == '"') inStr = false; }
                else if (c == '"') inStr = true;
                else if (c == open) depth++;
                else if (c == close) { depth--; if (depth == 0) return json.Substring(start, i - start + 1); }
            }
            return json.Substring(start);
        }

        private void ThrowIfDisposed()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(CavsClient));
        }

        public void Dispose()
        {
            lock (_lock)
            {
                if (_disposed) return;
                _disposed = true;
                if (_ctx != IntPtr.Zero)
                {
                    NativeMethods.cavs_context_free(_ctx);
                    _ctx = IntPtr.Zero;
                }
                _cbDelegate = null;
            }
        }
    }
}
