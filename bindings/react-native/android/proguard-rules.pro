# The JSI turbo module is reached from C++ through JNI, so R8 cannot see the
# references and would strip the entry points.
-keep class org.cashudevkit.crypto.** { *; }
-keep class com.facebook.jni.** { *; }
