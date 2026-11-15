package com.afzal.ayshamusic

import android.content.Context

/** Rust APK intentionally has no Media3 dependency or platform audio output. */
object PlatformAudio {
    fun initialize(context: Context) = Unit
    fun command(text: String) = Unit
}
