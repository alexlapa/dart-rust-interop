mod executor;

use std::{
    ffi::{CStr, CString},
    time::Duration,
};

use dart_sys::{Dart_Handle, Dart_PersistentHandle};
use extern_executor::spawn;
use futures_timer::Delay;

#[link(name = "trampoline")]
extern "C" {
    fn Dart_InitializeApiDL(obj: *mut libc::c_void) -> libc::intptr_t;
    fn Dart_NewPersistentHandle_DL_Trampolined(object: Dart_Handle) -> Dart_PersistentHandle;
    fn Dart_HandleFromPersistent_DL_Trampolined(object: Dart_PersistentHandle) -> Dart_Handle;
    fn Dart_DeletePersistentHandle_DL_Trampolined(object: Dart_PersistentHandle);
}

pub struct DartCallback {
    cb: Dart_PersistentHandle,
}

unsafe impl Send for DartCallback {}

impl DartCallback {
    pub fn new(cb: Dart_Handle) -> Self {
        DartCallback {
            cb: unsafe { Dart_NewPersistentHandle_DL_Trampolined(cb) },
        }
    }

    pub unsafe fn call(&self) {
        let closure_handle = Dart_HandleFromPersistent_DL_Trampolined(self.cb);
        closures_caller.unwrap()(closure_handle);
    }
}

impl Drop for DartCallback {
    fn drop(&mut self) {
        unsafe { Dart_DeletePersistentHandle_DL_Trampolined(self.cb) };
    }
}

#[no_mangle]
pub unsafe extern "C" fn InitDartApiDL(obj: *mut libc::c_void) -> libc::intptr_t {
    return Dart_InitializeApiDL(obj);
}

static mut closures_caller: Option<extern "C" fn(c: Dart_Handle)> = None;

/// simple callback
static mut callback: Option<DartCallback> = None;

#[no_mangle]
pub unsafe extern "C" fn RegisterClosureCallerFP(callback_: extern "C" fn(c: Dart_Handle)) {
    closures_caller = Some(callback_);
}

#[no_mangle]
pub unsafe extern "C" fn RegisterClosureCallback(cb: Dart_Handle) {
    callback = Some(DartCallback::new(cb));
}

#[no_mangle]
pub unsafe extern "C" fn InvokeClosureCallback() {
    callback.as_ref().unwrap().call();
}

/// async fn
#[no_mangle]
pub unsafe extern "C" fn RunAsync(timeout: i64, cb: Dart_Handle) {
    let cb = DartCallback::new(cb);

    spawn(async move {
        Delay::new(Duration::from_millis(timeout as u64)).await;
        cb.call();
    });
}

/// strings

#[no_mangle]
pub unsafe extern "C" fn Strings(string_in: *const libc::c_char) -> *const libc::c_char {
    let string_in = CStr::from_ptr(string_in).to_str().unwrap().to_owned();
    println!("Received string from Dart: {}", string_in);
    let reversed: String = string_in.chars().into_iter().rev().collect();
    CString::new(reversed).unwrap().into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn FreeRustString(s: *mut libc::c_char) {
    if s.is_null() {
        return;
    }
    CString::from_raw(s);
}
