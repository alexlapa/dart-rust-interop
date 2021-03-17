use dart_sys::{Dart_CObject, Dart_Port, Dart_Handle, Dart_PersistentHandle};

#[link(name = "trampoline")]
extern "C" {
    fn Dart_InitializeApiDL(obj: *mut libc::c_void) -> libc::intptr_t;
    fn Dart_NewPersistentHandle_DL_Trampolined(object: Dart_Handle) -> Dart_PersistentHandle;
    fn Dart_HandleFromPersistent_DL_Trampolined(object: Dart_PersistentHandle) -> Dart_Handle;
    fn Dart_DeletePersistentHandle_DL_Trampolined(object: Dart_PersistentHandle);
    fn Dart_PostCObject_DL_Trampolined(port_id: Dart_Port, message: *mut Dart_CObject) -> bool;
}

#[no_mangle]
pub unsafe extern "C" fn InitDartApiDL(obj: *mut libc::c_void) -> libc::intptr_t {
    return Dart_InitializeApiDL(obj);
}

/// simple callback

static mut closures_caller: Option<extern "C" fn(c: Dart_Handle)> = None;
static mut callback: Option<Dart_PersistentHandle> = None;

#[no_mangle]
pub unsafe extern "C" fn RegisterClosureCallerFP(callback_: extern "C" fn(c: Dart_Handle)) {
    closures_caller = Some(callback_);
}

#[no_mangle]
pub unsafe extern "C" fn RegisterClosureCallback(h: Dart_Handle) {
    callback = Some(Dart_NewPersistentHandle_DL_Trampolined(h));
}

#[no_mangle]
pub unsafe extern "C" fn InvokeClosureCallback() {
    let closure_handle = Dart_HandleFromPersistent_DL_Trampolined(callback.unwrap());
    closures_caller.unwrap()(closure_handle);
}

#[no_mangle]
pub unsafe extern "C" fn ReleaseClosureCallback() {
    Dart_DeletePersistentHandle_DL_Trampolined(callback.take().unwrap());
}
