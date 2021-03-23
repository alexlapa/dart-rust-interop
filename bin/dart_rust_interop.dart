import 'dart:async';
import 'dart:ffi';
import 'package:ffi/ffi.dart';

import 'executor.dart';

final nativeLib = DynamicLibrary.open('./rust/target/debug/librust_ffi.so');

void doDynamicLinking() {
  final nativeInitializeApi = nativeLib.lookupFunction<
      IntPtr Function(Pointer<Void>),
      int Function(Pointer<Void>)>("InitDartApiDL");

  if (nativeInitializeApi(NativeApi.initializeApiDLData) != 0) {
    throw "Failed to initialize Dart API";
  }

  nativeLib.lookupFunction<Void Function(Pointer), void Function(Pointer)>(
          "RegisterClosureCallerFP")(
      Pointer.fromFunction<Void Function(Handle)>(doClosureCallback));
}

void main() async {
  doDynamicLinking();
  var executor = Executor(nativeLib);
  executor.start();

  print("runAsync 1s start");
  await runAsync(1000);
  print("runAsync 1s done");
  print("runAsync 5s start");
  await runAsync(5000);
  print("runAsync 5s done");

  print("running simpleCallback");
  simpleCallback();

  assert(strings("123456789") == '987654321');

  try {
    throwFromNative();
  } catch (e, s) {
    print("Exception from Rust is caught `${e}`\n ${s}");
  } finally {
    print("finally block is executed when exception from rust is caught");
  }

  await callDartFutureFromRustFromDart();

  assert(Color.blue == enums(Color.rust));

  executor.stop();
}

/////////////////////// simple callback

final nativeRegisterClosureCallback = nativeLib.lookupFunction<
    Void Function(Handle),
    void Function(void Function())>("RegisterClosureCallback");

final nativeInvokeClosureCallback = nativeLib
    .lookupFunction<Void Function(), void Function()>("InvokeClosureCallback");

void doClosureCallback(void Function() callback) {
  callback();
}

void simpleCallback() {
  // C holds on to this closure through a `Dart_PersistentHandle`.
  nativeRegisterClosureCallback(() => {print('simpleCallback fires')});
  // Some time later this closure can be invoked.
  nativeInvokeClosureCallback();
}

//////////////////// async fn

final nativeRunAsync = nativeLib.lookupFunction<Void Function(Int64, Handle),
    void Function(int, void Function())>("RunAsync");

Future<void> runAsync(int) async {
  final Completer _completer = new Completer();
  nativeRunAsync(int, () => {_completer.complete()});

  return _completer.future;
}

//////////////// strings

final nativeStrings = nativeLib.lookupFunction<
    Pointer<Utf8> Function(Pointer<Utf8>),
    Pointer<Utf8> Function(Pointer<Utf8>)>("Strings");

String strings(String stringIn) {
  var stringPtr = stringIn.toNativeUtf8();
  try {
    var stringOut = nativeStrings(stringPtr);
    return stringOut.toDartString();
  } finally {
    calloc.free(stringPtr);
  }
}

final nativeFreeRustString = nativeLib.lookupFunction<
    Void Function(Pointer<Utf8>),
    void Function(Pointer<Utf8>)>("FreeRustString");

extension RustStringPointer on Pointer<Utf8> {
  String rustStringToDart() {
    try {
      return this.toDartString();
    } finally {
      nativeFreeRustString(this);
    }
  }
}

//////////////// throw from native

final nativeThrowFromNative = nativeLib
    .lookupFunction<Void Function(), void Function()>("ThrowFromNative");

void throwFromNative() {
  nativeThrowFromNative();
}

/////////////// call dart future from rust

final nativeOneshotSendOk = nativeLib.lookupFunction<
    Void Function(Pointer, Int64),
    void Function(Pointer, int)>("OneshotSendOk");
final nativeOneshotSendErr = nativeLib.lookupFunction<
    Void Function(Pointer, Int64),
    void Function(Pointer, int)>("OneshotSendErr");
final nativeCallDartFutureFromRust = nativeLib.lookupFunction<
    Void Function(Pointer, Handle),
    void Function(Pointer, void Function())>("CallDartFutureFromRust");

Future<void> callDartFutureFromRustFromDart() {
  final Completer _completer = new Completer();

  var fp =
      Pointer.fromFunction<Void Function(Pointer)>(wrapperForSomeDartAsyncFn);
  nativeCallDartFutureFromRust(fp, () => {_completer.complete()});

  return _completer.future;
}

Future<int> someDartAsyncFnThatWeWantToCallFromRust() async {
  return Future.delayed(Duration(seconds: 1), () => 333);
}

void wrapperForSomeDartAsyncFn(Pointer txPtr) {
  someDartAsyncFnThatWeWantToCallFromRust().then(
      (value) => {nativeOneshotSendOk(txPtr, value)},
      onError: (err, st) => {nativeOneshotSendErr(txPtr, err)});
}

//////////////// enums

enum Color { blue, rust }

final nativeEnums =
    nativeLib.lookupFunction<Uint8 Function(Uint8), int Function(int)>("Enums");

Color enums(Color color) {
  return Color.values[nativeEnums(color.index)];
}
