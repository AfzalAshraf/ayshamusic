package com.afzal.ayshamusic

import android.content.Context
import android.util.Log

/**
 * JNI bridge to the Rust library (libayshamusic.so).
 * 
 * The native function signature must match:
 *   JNIEXPORT void JNICALL Java_com_afzal_ayshamusic_RustBridge_startServer(
 *       JNIEnv* env, jobject receiver, jstring filesDir, jstring cacheDir, jobject context, jint port, jstring engine, jstring instance
 *   )
 */
object RustBridge {
    private const val TAG = "RustBridge"
    private var serverStarted = false

    // Bounded share parsing is implemented in Rust. Android retains intent delivery.
    external fun sharedVideoId(text: String): String?

    // Native method declaration
    @Suppress("UNUSED_PARAMETER")
    private external fun startServer(filesDir: String, cacheDir: String, context: Context, port: Int, engine: String, instance: String)

    /**
     * Starts the Rust HTTP server on the variant-specific loopback port.
     * Must be called once from MainActivity.onCreate().
     */
    fun startServer(context: Context) {
        if (serverStarted) {
            Log.w(TAG, "Server already started, ignoring duplicate call")
            return
        }

        val filesDir = context.filesDir.absolutePath
        val cacheDir = context.cacheDir.absolutePath

        Log.i(TAG, "Starting Rust server with filesDir=$filesDir, cacheDir=$cacheDir")
        
        try {
            // CPAL needs a JVM + long-lived application Context before audio opens.
            // Never retain an Activity: it may be destroyed while playback continues.
            PlatformAudio.initialize(context.applicationContext)
            startServer(filesDir, cacheDir, context.applicationContext, BuildConfig.API_PORT, BuildConfig.AUDIO_ENGINE, MediaApi.instanceId)
            serverStarted = true
            Log.i(TAG, "Rust server start request sent successfully")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Native library not loaded or JNI signature mismatch", e)
            throw RuntimeException("Failed to start Rust server: native library missing", e)
        }
    }

    /**
     * Checks if the server has been started.
     */
    @Suppress("unused")
    fun platformCommand(command: String) { PlatformAudio.command(command) }

    @Suppress("unused")
    fun squareCover(bytes: ByteArray): ByteArray? = AlbumArtwork.squareJpeg(bytes)

    @Suppress("unused")
    fun canPublishFavorites():Boolean {
        val context=AyshaMusicApplication.instance ?: return false
        return FavoritePublisher.canPublish(context)
    }

    @Suppress("unused")
    fun publishFavorite(path:String,id:String,title:String,artist:String):Boolean {
        val context=AyshaMusicApplication.instance ?: return false
        return FavoritePublisher.publish(context,path,id,title,artist)
    }

    fun isServerStarted(): Boolean = serverStarted
}