// UCavsClient — a Blueprint-friendly wrapper over the CAVS SDK C ABI for
// Unreal Engine. The high-value runtime operation is FetchStatic: install or
// update a build straight from a static CDN export (`cavs store export
// --static-plans`) with no game server, downloading only the chunks that
// changed, verified end to end.
//
// STATUS: untested. Compiles against the C ABI (cavs_sdk.h, ABI 1.0.0) and
// mirrors the shipping SDK bindings, but has not been built with UBT or run
// in-editor yet.

#pragma once

#include "CoreMinimal.h"
#include "UObject/NoExportTypes.h"
#include "CavsClient.generated.h"

/// Result of a serverless fetch.
USTRUCT(BlueprintType)
struct FCavsFetchResult
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "CAVS")
    int64 ChunksFetched = 0;

    UPROPERTY(BlueprintReadOnly, Category = "CAVS")
    int64 ChunksReused = 0;

    UPROPERTY(BlueprintReadOnly, Category = "CAVS")
    int64 WireBytes = 0;

    UPROPERTY(BlueprintReadOnly, Category = "CAVS")
    int64 LogicalBytes = 0;

    UPROPERTY(BlueprintReadOnly, Category = "CAVS")
    float SavedPercent = 0.f;
};

/// Parameters for a serverless fetch.
USTRUCT(BlueprintType)
struct FCavsFetchStaticRequest
{
    GENERATED_BODY()

    /// Base URL or local directory of the static export.
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    FString Base;

    /// Asset name (the `<name>` under `assets/<name>/`).
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    FString Asset;

    /// Output directory for the reconstructed build.
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    FString OutputDir;

    /// Persistent content-addressable cache directory.
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    FString CacheDir;

    /// Concurrent range requests (0 = default 8).
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    int32 Connections = 8;

    /// Optional Ed25519 public key (64 hex) enforcing the content signature.
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "CAVS")
    FString Pubkey;
};

DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FCavsProgress, float, Percentage, FString, Phase);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_OneParam(FCavsFetchCompleted, FCavsFetchResult, Result);
DECLARE_DYNAMIC_MULTICAST_DELEGATE_TwoParams(FCavsFetchFailed, FString, Code, FString, Message);

UCLASS(BlueprintType)
class CAVSSDK_API UCavsClient : public UObject
{
    GENERATED_BODY()

public:
    UCavsClient();
    virtual void BeginDestroy() override;

    /// The native SDK version (e.g. "1.4.0").
    UFUNCTION(BlueprintCallable, Category = "CAVS")
    static FString Version();

    /// Start a serverless install/update on a background thread. Progress,
    /// completion and failure are delivered on the game thread via the
    /// multicast delegates.
    UFUNCTION(BlueprintCallable, Category = "CAVS")
    void FetchStatic(const FCavsFetchStaticRequest& Request);

    /// Request cancellation of an in-flight FetchStatic.
    UFUNCTION(BlueprintCallable, Category = "CAVS")
    void Cancel();

    UPROPERTY(BlueprintAssignable, Category = "CAVS")
    FCavsProgress OnProgress;

    UPROPERTY(BlueprintAssignable, Category = "CAVS")
    FCavsFetchCompleted OnCompleted;

    UPROPERTY(BlueprintAssignable, Category = "CAVS")
    FCavsFetchFailed OnFailed;

private:
    void* Context = nullptr;                 // CavsContext*
    TAtomic<bool> bCancelRequested{ false };
    FString BuildRequestJson(const FCavsFetchStaticRequest& Request) const;
};
