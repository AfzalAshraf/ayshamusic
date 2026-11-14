package com.afzal.ayshamusic

import android.content.Context
import android.util.Log

/**
 * JNI bridge to the Rust library (libayshamusic.so).
 * 
 * The native function signature must match:
 *   JNIEXPORT void JNICALL Java_com_afzal_ayshamusic_RustBridge_startServer(
 *       JNIEnv* env, jclass clazz, jstring filesDir, jstring cacheDir
 *   )
 */
object RustBridge {
    private const val TAG = "RustBridge"
    private var serverStarted = false

    // Native method declaration
    @Suppress("UNUSED_PARAMETER")
    private external fun startServer(filesDir: String, cacheDir: String)

    /**
     * Starts the Rust HTTP server on 127.0.0.1:5050.
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
            startServer(filesDir, cacheDir)
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
    fun isServerStarted(): Boolean = serverStarted
}