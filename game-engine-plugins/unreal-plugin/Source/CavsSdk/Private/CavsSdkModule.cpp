#include "CavsSdkModule.h"
#include "Interfaces/IPluginManager.h"
#include "Misc/Paths.h"
#include "HAL/PlatformProcess.h"

#define LOCTEXT_NAMESPACE "FCavsSdkModule"

void FCavsSdkModule::StartupModule()
{
    // Resolve the platform library shipped under the plugin's Binaries/ or
    // ThirdParty lib dir; delay-loaded on Windows so we control the path.
    const FString BaseDir = IPluginManager::Get().FindPlugin(TEXT("CavsSdk"))->GetBaseDir();
#if PLATFORM_WINDOWS
    const FString LibPath = FPaths::Combine(*BaseDir, TEXT("Source/ThirdParty/CavsSdkLibrary/lib/Win64/cavs_sdk.dll"));
#elif PLATFORM_MAC
    const FString LibPath = FPaths::Combine(*BaseDir, TEXT("Source/ThirdParty/CavsSdkLibrary/lib/Mac/libcavs_sdk.dylib"));
#elif PLATFORM_LINUX
    const FString LibPath = FPaths::Combine(*BaseDir, TEXT("Source/ThirdParty/CavsSdkLibrary/lib/Linux/libcavs_sdk.so"));
#else
    const FString LibPath;
#endif

    if (!LibPath.IsEmpty())
    {
        LibraryHandle = FPlatformProcess::GetDllHandle(*LibPath);
        if (!LibraryHandle)
        {
            UE_LOG(LogTemp, Warning, TEXT("CAVS: failed to load native library at %s"), *LibPath);
        }
    }
}

void FCavsSdkModule::ShutdownModule()
{
    if (LibraryHandle)
    {
        FPlatformProcess::FreeDllHandle(LibraryHandle);
        LibraryHandle = nullptr;
    }
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FCavsSdkModule, CavsSdk)
