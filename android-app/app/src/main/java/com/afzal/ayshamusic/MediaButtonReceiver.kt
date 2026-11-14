package com.afzal.ayshamusic

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.view.KeyEvent

/**
 * Routes Bluetooth / wired headset media buttons to the foreground service.
 */
class MediaButtonReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_MEDIA_BUTTON) return

        @Suppress("DEPRECATION")
        val event = intent.getParcelableExtra<KeyEvent>(Intent.EXTRA_KEY_EVENT)
            ?: return
        if (event.action != KeyEvent.ACTION_DOWN) return

        val action = when (event.keyCode) {
            KeyEvent.KEYCODE_MEDIA_PLAY -> MusicForegroundService.ACTION_PLAY
            KeyEvent.KEYCODE_MEDIA_PAUSE -> MusicForegroundService.ACTION_PAUSE
            KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE -> MusicForegroundService.ACTION_PLAY
            KeyEvent.KEYCODE_MEDIA_NEXT -> MusicForegroundService.ACTION_NEXT
            KeyEvent.KEYCODE_MEDIA_PREVIOUS -> MusicForegroundService.ACTION_PREV
            KeyEvent.KEYCODE_MEDIA_STOP -> MusicForegroundService.ACTION_STOP
            else -> return
        }

        context.startService(
            Intent(context, MusicForegroundService::class.java).setAction(action)
        )
    }
}