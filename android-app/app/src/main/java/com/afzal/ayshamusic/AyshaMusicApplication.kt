package com.afzal.ayshamusic

import android.app.Application
import android.content.Context
import android.util.Log

class AyshaMusicApplication : Application() {
    companion object {
        private const val TAG = "AyshaMusicApp"
        @Suppress("UNUSED_PROPERTY")
        var instance: AyshaMusicApplication? = null
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        
        // Load native library as early as possible
        try {
            System.loadLibrary("ayshamusic")
            Log.i(TAG, "libayshamusic.so loaded successfully")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load libayshamusic.so", e)
        }
    }

    fun getFilesDirPath(): String = filesDir.absolutePath
    fun getCacheDirPath(): String = cacheDir.absolutePath
}