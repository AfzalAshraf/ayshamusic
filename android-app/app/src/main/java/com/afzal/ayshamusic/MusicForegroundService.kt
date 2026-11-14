package com.afzal.ayshamusic

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.media.app.NotificationCompat.MediaStyle

/**
 * Foreground media service.
 *
 * Responsibilities (no playback itself - Rust/rodio owns the audio):
 *  - foregroundServiceType="mediaPlayback" notification
 *  - partial WakeLock while playing
 *  - AudioManager focus + ACTION_AUDIO_BECOMING_NOISY (pause on headphone unplug)
 *  - MediaSessionCompat lock-screen / Bluetooth transport controls
 */
class MusicForegroundService : Service() {
    companion object {
        private const val TAG = "MusicService"
        private const val CHANNEL_MEDIA = "media_playback"
        private const val CHANNEL_SYNC = "telegram_sync"
        private const val NOTIFICATION_ID = 42

        const val ACTION_PLAY = "com.afzal.ayshamusic.PLAY"
        const val ACTION_PAUSE = "com.afzal.ayshamusic.PAUSE"
        const val ACTION_NEXT = "com.afzal.ayshamusic.NEXT"
        const val ACTION_PREV = "com.afzal.ayshamusic.PREV"
        const val ACTION_STOP = "com.afzal.ayshamusic.STOP"

        fun start(context: Context) {
            val intent = Intent(context, MusicForegroundService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }
    }

    private var mediaSession: MediaSessionCompat? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var audioFocusRequest: AudioFocusRequest? = null
    private var playing = false
    private var currentTitle = "Aysha's Music"
    private var currentArtist = ""

    private val becomingNoisy = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) {
                pause()
            }
        }
    }

    private val audioFocusListener = AudioManager.OnAudioFocusChangeListener { change ->
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS,
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> pause()
            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> Unit
        }
    }

    override fun onCreate() {
        super.onCreate()
        createChannels()
        setupMediaSession()
        registerReceiver(
            becomingNoisy,
            IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY)
        )
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_PLAY -> play()
            ACTION_PAUSE -> pause()
            ACTION_NEXT -> MediaApi.post("/api/next")
            ACTION_PREV -> MediaApi.post("/api/prev")
            ACTION_STOP -> stopPlayback()
            else -> {
                startForeground(NOTIFICATION_ID, buildNotification())
                updateState(false)
            }
        }
        return START_STICKY
    }

    private fun setupMediaSession() {
        mediaSession = MediaSessionCompat(this, TAG).apply {
            setCallback(object : MediaSessionCompat.Callback() {
                override fun onPlay() = play()
                override fun onPause() = pause()
                override fun onSkipToNext() = MediaApi.post("/api/next")
                override fun onSkipToPrevious() = MediaApi.post("/api/prev")
                override fun onStop() = stopPlayback()
            })
            setFlags(
                MediaSessionCompat.FLAG_HANDLES_MEDIA_BUTTONS or
                    MediaSessionCompat.FLAG_HANDLES_TRANSPORT_CONTROLS
            )
            isActive = true
        }
    }

    private fun play() {
        acquireWakeLock()
        requestAudioFocus()
        playing = true
        MediaApi.post("/api/resume")
        MediaApi.post("/api/play")
        startForeground(NOTIFICATION_ID, buildNotification())
        updateState(true)
    }

    private fun pause() {
        playing = false
        MediaApi.post("/api/pause")
        releaseWakeLock()
        updateState(false)
        startForeground(NOTIFICATION_ID, buildNotification())
    }

    private fun stopPlayback() {
        playing = false
        MediaApi.post("/api/stop")
        releaseWakeLock()
        abandonAudioFocus()
        updateState(false)
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    /** Called by MainActivity/JS bridge when metadata changes. */
    fun updateMetadata(title: String, artist: String) {
        currentTitle = title.ifBlank { "Aysha's Music" }
        currentArtist = artist
        mediaSession?.setMetadata(
            MediaMetadataCompat.Builder()
                .putString(MediaMetadataCompat.METADATA_KEY_TITLE, currentTitle)
                .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, currentArtist)
                .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, "Aysha's Music")
                .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, -1L)
                .build()
        )
        if (playing) startForeground(NOTIFICATION_ID, buildNotification())
        updateState(playing)
    }

    private fun updateState(isPlaying: Boolean) {
        val state = if (isPlaying) PlaybackStateCompat.STATE_PLAYING else PlaybackStateCompat.STATE_PAUSED
        mediaSession?.setPlaybackState(
            PlaybackStateCompat.Builder()
                .setActions(
                    PlaybackStateCompat.ACTION_PLAY or
                        PlaybackStateCompat.ACTION_PAUSE or
                        PlaybackStateCompat.ACTION_PLAY_PAUSE or
                        PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                        PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                        PlaybackStateCompat.ACTION_STOP
                )
                .setState(state, PlaybackStateCompat.PLAYBACK_POSITION_UNKNOWN, 1f)
                .build()
        )
    }

    private fun buildNotification(): Notification {
        val contentIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        fun action(icon: Int, title: String, actionName: String, requestCode: Int): NotificationCompat.Action {
            val intent = Intent(this, MusicForegroundService::class.java).setAction(actionName)
            val pending = PendingIntent.getService(
                this, requestCode, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            return NotificationCompat.Action(icon, title, pending)
        }

        return NotificationCompat.Builder(this, CHANNEL_MEDIA)
            .setSmallIcon(R.drawable.ic_stat_music)
            .setContentTitle(currentTitle)
            .setContentText(currentArtist.ifBlank { if (playing) "Playing" else "Paused" })
            .setContentIntent(contentIntent)
            .setOngoing(playing)
            .setOnlyAlertOnce(true)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .addAction(action(R.drawable.ic_stat_previous, "Previous", ACTION_PREV, 1))
            .addAction(
                action(
                    if (playing) R.drawable.ic_stat_pause else R.drawable.ic_stat_play,
                    if (playing) "Pause" else "Play",
                    if (playing) ACTION_PAUSE else ACTION_PLAY,
                    2
                )
            )
            .addAction(action(R.drawable.ic_stat_next, "Next", ACTION_NEXT, 3))
            .setStyle(MediaStyle().setMediaSession(mediaSession?.sessionToken).setShowActionsInCompactView(0, 1, 2))
            .build()
    }

    private fun createChannels() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_MEDIA, getString(R.string.notification_channel_media), NotificationManager.IMPORTANCE_LOW).apply {
                setShowBadge(false)
                lockscreenVisibility = Notification.VISIBILITY_PUBLIC
            }
        )
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_SYNC, getString(R.string.notification_channel_sync), NotificationManager.IMPORTANCE_MIN).apply {
                setShowBadge(false)
            }
        )
    }

    private fun acquireWakeLock() {
        if (wakeLock?.isHeld == true) return
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "ayshamusic:playback").apply {
            setReferenceCounted(false)
            acquire(4 * 60 * 60 * 1000L)
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
    }

    private fun requestAudioFocus() {
        val audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build()
                )
                .setOnAudioFocusChangeListener(audioFocusListener)
                .build()
                .also { audioManager.requestAudioFocus(it) }
        } else {
            @Suppress("DEPRECATION")
            audioManager.requestAudioFocus(
                audioFocusListener,
                AudioManager.STREAM_MUSIC,
                AudioManager.AUDIOFOCUS_GAIN
            )
        }
    }

    private fun abandonAudioFocus() {
        val audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest?.let { audioManager.abandonAudioFocusRequest(it) }
            audioFocusRequest = null
        } else {
            @Suppress("DEPRECATION")
            audioManager.abandonAudioFocus(audioFocusListener)
        }
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        // Keep music playing when the task is swiped away
        super.onTaskRemoved(rootIntent)
    }

    override fun onDestroy() {
        try {
            unregisterReceiver(becomingNoisy)
        } catch (_: IllegalArgumentException) {
            // already unregistered
        }
        releaseWakeLock()
        abandonAudioFocus()
        mediaSession?.apply {
            isActive = false
            release()
        }
        mediaSession = null
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}