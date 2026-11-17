package com.afzal.ayshamusic

import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import org.json.JSONObject

/** Audio only. Rust retains extraction, provider HTTP and Telegram credentials. */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
object PlatformAudio {
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var context: Context
    private var player: ExoPlayer? = null
    private var token = ""
    private var failed = false
    private var revision = 0L

    fun initialize(context: Context) { this.context = context.applicationContext }

    fun command(text: String) {
        handler.post {
            try {
                val command = JSONObject(text)
                val incoming = command.getString("token")
                if (command.getString("op") == "play") {
                    releaseOutput()
                    token = incoming
                    revision = command.optLong("revision", 0L)
                    failed = false
                    val uri = android.net.Uri.parse(command.getString("url"))
                    require(uri.scheme == "http" && uri.host == "127.0.0.1" &&
                        uri.port == BuildConfig.API_PORT && uri.path == "/api/platform/media/$incoming")
                    // Rust permits 12s per upstream block and the input worker
                    // waits at most15s. Media3's default8s would abort a healthy
                    // slower block first. This is inactivity, not song duration.
                    val http = DefaultHttpDataSource.Factory()
                        .setConnectTimeoutMs(8000)
                        .setReadTimeoutMs(20000)
                    val audio = ExoPlayer.Builder(context)
                        .setMediaSourceFactory(DefaultMediaSourceFactory(DefaultDataSource.Factory(context, http)))
                        .setLoadControl(DefaultLoadControl.Builder()
                            .setBufferDurationsMs(15000, 30000, 1000, 2000).build())
                        .build()
                    player = audio
                    audio.setAudioAttributes(AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(), true)
                    audio.setHandleAudioBecomingNoisy(true)
                    audio.setWakeMode(C.WAKE_MODE_LOCAL)
                    audio.volume = command.optDouble("volume", 1.0).toFloat().coerceIn(0f, 1f)
                    audio.addListener(object : Player.Listener {
                        override fun onEvents(source: Player, events: Player.Events) {
                            if (player === audio && !failed) report()
                        }
                        override fun onPlayerError(error: PlaybackException) {
                            if (player !== audio) return
                            failed = true
                            // Never send signed URLs or credentials into status/logs.
                            report("error", "media3_${error.errorCodeName}: ${error.cause?.javaClass?.simpleName ?: "playback_failed"}")
                        }
                    })
                    audio.setMediaItem(MediaItem.fromUri(uri))
                    audio.prepare()
                    audio.playWhenReady = command.optBoolean("play_when_ready", true)
                    handler.removeCallbacks(tick)
                    handler.post(tick)
                } else if (incoming == token) {
                    if (command.has("revision")) {
                        val next = command.getLong("revision")
                        if (next < revision) return@post
                        revision = next
                    }
                    when (command.getString("op")) {
                        "pause" -> player?.pause()
                        "resume" -> player?.play()
                        "volume" -> player?.volume = command.optDouble("volume", 1.0).toFloat().coerceIn(0f, 1f)
                        "stop" -> {
                            releaseOutput()
                        }
                    }
                    if (player != null && !failed) report()
                }
            } catch (_: Exception) {
                failed = true
                report("error", "media3_command_failed")
            }
        }
    }

    private fun releaseOutput() {
        val old = player
        player = null
        token = ""
        handler.removeCallbacks(tick)
        old?.release()
    }

    private val tick = object : Runnable {
        override fun run() {
            if (player == null || token.isEmpty()) return
            if (!failed) report()
            handler.postDelayed(this, 1000)
        }
    }

    private fun report(overrideState: String? = null, error: String? = null) {
        val audio = player ?: return
        if (token.isEmpty()) return
        val state = overrideState ?: when {
            audio.playbackState == Player.STATE_ENDED -> "ended"
            !audio.playWhenReady -> "paused"
            audio.playbackState == Player.STATE_BUFFERING -> "buffering"
            audio.isPlaying -> "playing"
            else -> "paused"
        }
        val event = JSONObject().put("token", token).put("revision", revision).put("state", state)
            .put("position", audio.currentPosition.coerceAtLeast(0) / 1000.0)
            .put("duration", audio.duration.coerceAtLeast(0) / 1000.0)
            .put("ready", audio.playbackState == Player.STATE_READY)
        if (error != null) event.put("error", error)
        MediaApi.post("/api/platform/event", event.toString())
    }
}
