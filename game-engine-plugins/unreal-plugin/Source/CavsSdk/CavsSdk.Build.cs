// Unreal Build Tool module rules for the CAVS SDK plugin.
//
// Links the prebuilt CAVS native library (libcavs_sdk) via the C ABI header
// vendored under Source/ThirdParty/CavsSdkLibrary. Drop the platform library
// artifact from the `cavs-sdk-native-*` release into the matching folder
// (see the plugin README).
//
// STATUS: untested — mirrors the standard Unreal third-party linking pattern
// but has not been built with UBT yet.

using System.IO;
using UnrealBuildTool;

public class CavsSdk : ModuleRules
{
    public CavsSdk(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(new string[] { "Core" });
        PrivateDependencyModuleNames.AddRange(new string[] { "CoreUObject", "Engine", "Json" });

        string ThirdParty = Path.Combine(ModuleDirectory, "..", "ThirdParty", "CavsSdkLibrary");
        PublicIncludePaths.Add(Path.Combine(ThirdParty, "include"));

        string LibDir = Path.Combine(ThirdParty, "lib", Target.Platform.ToString());
        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibDir, "cavs_sdk.dll.lib"));
            RuntimeDependencies.Add(Path.Combine(LibDir, "cavs_sdk.dll"));
            PublicDelayLoadDLLs.Add("cavs_sdk.dll");
        }
        else if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibDir, "libcavs_sdk.dylib"));
            RuntimeDependencies.Add(Path.Combine(LibDir, "libcavs_sdk.dylib"));
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            PublicAdditionalLibraries.Add(Path.Combine(LibDir, "libcavs_sdk.so"));
            RuntimeDependencies.Add(Path.Combine(LibDir, "libcavs_sdk.so"));
        }
    }
}
