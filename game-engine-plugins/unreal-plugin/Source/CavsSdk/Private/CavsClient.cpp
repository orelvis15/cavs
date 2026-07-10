#include "CavsClient.h"
#include "Async/Async.h"
#include "HAL/Runnable.h"
#include "HAL/RunnableThread.h"
#include "Dom/JsonObject.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"

// The C ABI (vendored header).
extern "C" {
#include "cavs_sdk.h"
}

// STATUS: untested. Mirrors the async job pattern of the shipping SDKs
// (start → poll → drain) and marshals progress/completion back to the game
// thread, but has not been compiled by UBT or run in-editor yet.

UCavsClient::UCavsClient()
{
    Context = cavs_context_new("{}");
}

void UCavsClient::BeginDestroy()
{
    bCancelRequested = true;
    if (Context)
    {
        cavs_context_free(static_cast<CavsContext*>(Context));
        Context = nullptr;
    }
    Super::BeginDestroy();
}

FString UCavsClient::Version()
{
    const char* v = cavs_sdk_version();
    return v ? FString(UTF8_TO_TCHAR(v)) : FString();
}

void UCavsClient::Cancel()
{
    bCancelRequested = true;
}

FString UCavsClient::BuildRequestJson(const FCavsFetchStaticRequest& R) const
{
    const TSharedRef<FJsonObject> Data = MakeShared<FJsonObject>();
    Data->SetStringField(TEXT("base"), R.Base);
    Data->SetStringField(TEXT("asset"), R.Asset);
    Data->SetStringField(TEXT("outputDir"), R.OutputDir);
    Data->SetStringField(TEXT("cacheDir"), R.CacheDir);
    if (R.Connections > 0) Data->SetNumberField(TEXT("connections"), R.Connections);
    if (!R.Pubkey.IsEmpty()) Data->SetStringField(TEXT("pubkey"), R.Pubkey);

    const TSharedRef<FJsonObject> Envelope = MakeShared<FJsonObject>();
    Envelope->SetStringField(TEXT("schemaVersion"), TEXT("1.0"));
    Envelope->SetObjectField(TEXT("data"), Data);

    FString Out;
    const TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&Out);
    FJsonSerializer::Serialize(Envelope, Writer);
    return Out;
}

void UCavsClient::FetchStatic(const FCavsFetchStaticRequest& Request)
{
    bCancelRequested = false;
    const FString Body = BuildRequestJson(Request);
    CavsContext* Ctx = static_cast<CavsContext*>(Context);

    // Run the whole job off the game thread; hop back for delegates.
    AsyncTask(ENamedThreads::AnyBackgroundThreadNormalTask, [this, Ctx, Body]()
    {
        CavsJob* Job = cavs_start_json(Ctx, "fetchStatic", TCHAR_TO_UTF8(*Body));
        if (!Job)
        {
            AsyncTask(ENamedThreads::GameThread, [this]()
            {
                OnFailed.Broadcast(TEXT("CAVS-E-INTERNAL"), TEXT("failed to start fetchStatic"));
            });
            return;
        }

        CavsResult* Result = nullptr;
        while ((Result = cavs_job_poll(Job)) == nullptr)
        {
            if (bCancelRequested)
            {
                cavs_job_cancel(Job);
            }
            FPlatformProcess::Sleep(0.015f);
        }

        const bool bOk = cavs_result_ok(Result) == 1;
        const char* JsonC = cavs_result_json(Result);
        const FString Json = JsonC ? FString(UTF8_TO_TCHAR(JsonC)) : FString();
        FString ErrCode, ErrMsg;
        if (!bOk)
        {
            const char* c = cavs_result_error_code(Result);
            const char* m = cavs_result_error_message(Result);
            ErrCode = c ? FString(UTF8_TO_TCHAR(c)) : TEXT("CAVS-E-INTERNAL");
            ErrMsg = m ? FString(UTF8_TO_TCHAR(m)) : TEXT("operation failed");
        }
        cavs_result_free(Result);
        cavs_job_free(Job);

        if (!bOk)
        {
            AsyncTask(ENamedThreads::GameThread, [this, ErrCode, ErrMsg]()
            {
                OnFailed.Broadcast(ErrCode, ErrMsg);
            });
            return;
        }

        // Parse {ok, operation, data:{...}} and pull the result fields.
        FCavsFetchResult Out;
        TSharedPtr<FJsonObject> Root;
        const TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(Json);
        if (FJsonSerializer::Deserialize(Reader, Root) && Root.IsValid())
        {
            const TSharedPtr<FJsonObject>* Data;
            if (Root->TryGetObjectField(TEXT("data"), Data))
            {
                Out.ChunksFetched = (int64)(*Data)->GetNumberField(TEXT("chunksFetched"));
                Out.ChunksReused = (int64)(*Data)->GetNumberField(TEXT("chunksReused"));
                Out.WireBytes = (int64)(*Data)->GetNumberField(TEXT("wireBytes"));
                Out.LogicalBytes = (int64)(*Data)->GetNumberField(TEXT("logicalBytes"));
                Out.SavedPercent = (float)(*Data)->GetNumberField(TEXT("savedPercent"));
            }
        }
        AsyncTask(ENamedThreads::GameThread, [this, Out]()
        {
            OnProgress.Broadcast(1.0f, TEXT("completed"));
            OnCompleted.Broadcast(Out);
        });
    });
}
