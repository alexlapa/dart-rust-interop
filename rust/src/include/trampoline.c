#include <stdlib.h>
#include "./dart_api_dl.c"

Dart_PersistentHandle Dart_NewPersistentHandle_DL_Trampolined(Dart_Handle handle)
{
    return Dart_NewPersistentHandle_DL(handle);
}

Dart_Handle Dart_HandleFromPersistent_DL_Trampolined(Dart_PersistentHandle handle)
{
    return Dart_HandleFromPersistent_DL(handle);
}

void Dart_DeletePersistentHandle_DL_Trampolined(Dart_PersistentHandle handle)
{
    Dart_DeletePersistentHandle_DL(handle);
}

bool Dart_PostCObject_DL_Trampolined(Dart_Port port_id, Dart_CObject* message)
{
    return Dart_PostCObject_DL(port_id, message);
}
