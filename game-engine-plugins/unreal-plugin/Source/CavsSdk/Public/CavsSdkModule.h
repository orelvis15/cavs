#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

/// The CAVS SDK runtime module. Loads the native library on startup and
/// unloads it on shutdown.
class FCavsSdkModule : public IModuleInterface
{
public:
    virtual void StartupModule() override;
    virtual void ShutdownModule() override;

private:
    /** Handle to the dynamically loaded cavs_sdk library. */
    void* LibraryHandle = nullptr;
};
