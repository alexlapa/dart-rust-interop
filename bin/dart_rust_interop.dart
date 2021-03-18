import 'dart:async';
import 'dart:ffi';

import 'executor.dart';

final nativeLib = DynamicLibrary.open('./rust/target/debug/librust_ffi.so');

void doDynamicLinking() {
  final initializeApi = nativeLib.lookupFunction<IntPtr Function(Pointer<Void>),
      int Function(Pointer<Void>)>("InitDartApiDL");

  if (initializeApi(NativeApi.initializeApiDLData) != 0) {
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

  executor.stop();
}

/////////////////////// simple callback

final registerClosureCallback = nativeLib.lookupFunction<Void Function(Handle),
    void Function(void Function())>("RegisterClosureCallback");

final invokeClosureCallback = nativeLib
    .lookupFunction<Void Function(), void Function()>("InvokeClosureCallback");
final releaseClosureCallback = nativeLib
    .lookupFunction<Void Function(), void Function()>("ReleaseClosureCallback");

void doClosureCallback(void Function() callback) {
  callback();
}

void simpleCallback() {
  // C holds on to this closure through a `Dart_PersistentHandle`.
  registerClosureCallback(() => {print('simpleCallback fires')});
  // Some time later this closure can be invoked.
  invokeClosureCallback();
  // // When C is done it needs to stop holding on to the closure such that the
  // // Dart GC can collect the closure.
  releaseClosureCallback();
}

//////////////////// async fn

final runNativeAsync = nativeLib.lookupFunction<Void Function(Int64, Handle),void Function(int, void Function())>("RunAsync");

void runAsync(int) async {
  final Completer _completer = new Completer();
  runNativeAsync(int, () => {
    _completer.complete()
  });

  return _completer.future;
}
