// P/Invoke bindings for the CAVS SDK C ABI (cavs_sdk.h, ABI 1.0.0).
//
// The native library ships per platform as:
//   Linux/BSD : libcavs_sdk.so     macOS : libcavs_sdk.dylib     Windows : cavs_sdk.dll
// Place the matching artifact from the `sdk-native` release under Plugins/.
// DllImport uses the base name "cavs_sdk"; Unity resolves the platform prefix.
//
// STATUS: untested. These signatures mirror the C header exactly, but the
// binding has not yet been exercised on a device.

using System;
using System.Runtime.InteropServices;

namespace Cavs
{
    internal static class NativeMethods
    {
        private const string Lib = "cavs_sdk";

        // Progress callback: (event_json, user_data).
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ProgressCallback(IntPtr eventJson, IntPtr userData);

        // --- Version / capabilities (static strings; do NOT free version ones) ---
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_sdk_version();

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_sdk_abi_version();

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_sdk_capabilities_json();

        // --- Context ---
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_context_new(string optionsJson);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void cavs_context_free(IntPtr ctx);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int cavs_context_set_progress_callback(IntPtr ctx, ProgressCallback callback, IntPtr userData);

        // --- Synchronous / async execution ---
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_execute_json(IntPtr ctx, string operation, string requestJson);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_start_json(IntPtr ctx, string operation, string requestJson);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_job_poll(IntPtr job);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int cavs_job_cancel(IntPtr job);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void cavs_job_free(IntPtr job);

        // --- Result accessors ---
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_result_json(IntPtr result);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern int cavs_result_ok(IntPtr result);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_result_error_code(IntPtr result);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr cavs_result_error_message(IntPtr result);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void cavs_result_free(IntPtr result);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        public static extern void cavs_string_free(IntPtr ptr);

        /// Marshal a NUL-terminated UTF-8 C string to a managed string.
        public static string Utf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            int len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            var bytes = new byte[len];
            Marshal.Copy(ptr, bytes, 0, len);
            return System.Text.Encoding.UTF8.GetString(bytes);
        }
    }
}
