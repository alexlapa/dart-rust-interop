mod executor;

use futures::channel::oneshot;

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
    fn Dart_NewApiError_DL_Trampolined(msg: *const libc::c_char) -> Dart_Handle;
    fn Dart_NewUnhandledExceptionError_DL_Trampolined(exception: Dart_Handle) -> Dart_Handle;
    fn Dart_PropagateError_DL_Trampolined(handle: Dart_Handle);
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

/// throw from native
#[no_mangle]
pub unsafe extern "C" fn ThrowFromNative() {
    let err_msg = "Exception thrown from rust message\0".as_ptr() as *const libc::c_char;
    let api_error = Dart_NewApiError_DL_Trampolined(err_msg);
    let unhandled_exception = Dart_NewUnhandledExceptionError_DL_Trampolined(api_error);
    // CAUTION: transfers control non-locally using a setjmp-like mechanism. None of the Rust code
    // after this statement is executed (including Drop's).
    Dart_PropagateError_DL_Trampolined(unhandled_exception);
    unreachable!("Actually unreachable");
}

/////////////// call dart async fn from rust

#[no_mangle]
pub unsafe extern "C" fn CallDartFutureFromRust(
    get_future: extern "C" fn(tx: *mut oneshot::Sender<Result<i64, i64>>),
    completion_cb: Dart_Handle,
) {
    let completion_cb = DartCallback::new(completion_cb);
    let (tx, rx) = oneshot::channel();
    get_future(Box::into_raw(Box::new(tx)));
    spawn(async move {
        println!("Dart future resolved with: {:?}", rx.await);
        completion_cb.call();
    });
}

#[no_mangle]
pub unsafe extern "C" fn OneshotSendOk(tx: *mut oneshot::Sender<Result<i64, i64>>, ok: i64) {
    let mut tx = unsafe { Box::from_raw(tx) };
    tx.send(Ok(ok));
}

#[no_mangle]
pub unsafe extern "C" fn OneshotSendErr(tx: *mut oneshot::Sender<Result<i64, i64>>, err: i64) {
    let mut tx = unsafe { Box::from_raw(tx) };
    tx.send(Err(err));
}

/////////////// enums

#[derive(Debug, Eq, PartialEq)]
enum Color {
    Blue,
    Rust,
}

impl From<u8> for Color {
    fn from(val: u8) -> Self {
        match val {
            0 => Color::Blue,
            1 => Color::Rust,
            _ => unreachable!(),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Enums(color: u8) -> libc::c_int {
    assert_eq!(Color::from(color), Color::Blue);
    Color::Rust as libc::c_int
}

/////////////// arrays

#[repr(C)]
pub struct Array {
    arr: *const i64,
    len: u64,
}

impl From<Vec<i64>> for Array {
    fn from(mut vec: Vec<i64>) -> Self {
        Self {
            len: vec.len() as u64,
            arr: Box::leak(vec.into_boxed_slice()).as_ptr(),
        }
    }
}

impl Drop for Array {
    fn drop(&mut self) {
        if !self.arr.is_null() {
            unsafe {
                let slice = std::slice::from_raw_parts_mut(self.arr as *mut i64, self.len as usize);
                Box::from_raw(slice);
            };
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn Arrays() -> Array {
    Array::from(vec![1, 2, 3])
}

#[no_mangle]
pub unsafe extern "C" fn FreeArray(arr: Array) {}
