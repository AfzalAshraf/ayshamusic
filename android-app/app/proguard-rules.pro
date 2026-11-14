# Keep JNI bridge entry points
-keep class com.afzal.ayshamusic.RustBridge {
    native static void startServer(java.lang.String, java.lang.String);
    static void startServer(android.content.Context);
}

# Keep MainActivity and Service entry points
-keep class com.afzal.ayshamusic.MainActivity { *; }
-keep class com.afzal.ayshamusic.MusicForegroundService { *; }
-keep class com.afzal.ayshamusic.MediaButtonReceiver { *; }

# Keep WebView JavaScript interface if any
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# Views inflated from XML need their constructors kept (resource-shrinking safety)
-keep public class * extends android.view.View {
    public <init>(android.content.Context);
    public <init>(android.content.Context, android.util.AttributeSet);
    public <init>(android.content.Context, android.util.AttributeSet, int);
}

# Optimize aggressively
-optimizations !code/simplification/arithmetic,!code/simplification/cast,!code/simplification/comparison
-optimizationpasses 5
-allowaccessmodification
-dontusemixedcaseclassnames

# Strip line numbers and source file info (saves space)
-renamesourcefileattribute SourceFile
-keepattributes SourceFile,LineNumberTable

# Remove logging in release
-assumenosideeffects class android.util.Log {
    public static *** d(...);
    public static *** v(...);
    public static *** i(...);
    public static *** w(...);
}