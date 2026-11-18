package com.afzal.ayshamusic

import android.os.SystemClock
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.io.Closeable
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/** Actual production PlatformAudio/ExoPlayer on an Android emulator.
 * A local HTTP fixture replaces Rust/provider acquisition ONLY for these tests.
 * This does not prove YouTube access, physical speaker sound or a phone install.
 */
@RunWith(AndroidJUnit4::class)
class Media3PlaybackTest {
    private lateinit var server: FixtureServer
    private val token = "fixture"

    @Before fun setup() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val data = instrumentation.context.assets.open("tone.m4a").use { it.readBytes() }
        server = FixtureServer(data)
        server.start()
        PlatformAudio.initialize(instrumentation.targetContext)
        assertTrue("Fixture must satisfy the real backend identity check", MediaApi.verifyBackend())
    }

    @After fun cleanup() {
        if (::server.isInitialized) {
            command("stop", 99)
            InstrumentationRegistry.getInstrumentation().runOnMainSync { }
            server.close()
        }
    }

    private fun command(op: String, revision: Int, autoplay: Boolean = true) {
        val value = JSONObject().put("op", op).put("token", token).put("revision", revision)
        if (op == "play") value.put("url", "${MediaApi.baseUrl}/api/platform/media/$token")
            .put("volume", 0.2).put("play_when_ready", autoplay)
        PlatformAudio.command(value.toString())
    }

    private fun awaitEvent(timeout: Long = 15000, allowError: Boolean = false, predicate: (JSONObject) -> Boolean): JSONObject {
        val deadline = SystemClock.elapsedRealtime() + timeout
        while (SystemClock.elapsedRealtime() < deadline) {
            val event = server.events.poll(250, TimeUnit.MILLISECONDS) ?: continue
            if (event.optString("state") == "error" && !allowError) fail("Actual Media3 error: ${event.optString("error")}")
            if (predicate(event)) return event
        }
        throw AssertionError("Media3 fixture event deadline exceeded")
    }

    @Test fun actualMedia3ContinuesPastTwoSecondsPausesResumesAndEnds() {
        command("play", 0)
        awaitEvent { it.optString("state") == "playing" && it.optDouble("position") >= 3.0 }
        command("pause", 1)
        val paused = awaitEvent { it.optLong("revision") == 1L && it.optString("state") == "paused" }
        SystemClock.sleep(1200)
        server.events.clear() // inspect a fresh position sample, not an old queued pause
        val stillPaused = awaitEvent { it.optLong("revision") == 1L && it.optString("state") == "paused" }
        assertTrue("Pause must stop position advancement", stillPaused.optDouble("position") <= paused.optDouble("position") + 0.35)
        command("resume", 2)
        awaitEvent { it.optLong("revision") == 2L && it.optString("state") == "playing" && it.optDouble("position") > paused.optDouble("position") + 0.5 }
        awaitEvent(20000) { it.optString("state") == "ended" }
        assertTrue("Media3 must read actual fixture bytes", server.mediaRequests.get() > 0)
    }

    @Test fun pauseBeforeReadyIsHonoredThenResumeActuallyPlays() {
        command("play", 0, autoplay = false)
        val ready = awaitEvent { it.optBoolean("ready") && it.optString("state") == "paused" }
        assertTrue(ready.optDouble("position") < 0.25)
        command("resume", 1)
        awaitEvent { it.optLong("revision") == 1L && it.optString("state") == "playing" && it.optDouble("position") >= 2.5 }
    }

    @Test fun rejectedInputReportsErrorInsteadOfFalsePlaying() {
        server.rejectMedia = true
        command("play", 0)
        val error = awaitEvent(20000, allowError = true) { it.optString("state") == "error" }
        assertTrue(error.optString("error").startsWith("media3_"))
        assertTrue(error.optDouble("position") < 0.25)
    }

    @Test fun seekForwardAndBackwardWhilePausedThenResume() {
        server.data = InstrumentationRegistry.getInstrumentation().context.assets.open("long-tone.m4a").use { it.readBytes() }
        command("play", 0, autoplay = false)
        awaitEvent { it.optBoolean("ready") && it.optString("state") == "paused" }
        PlatformAudio.command(JSONObject().put("op","seek").put("token",token).put("revision",1).put("position",400.0).toString())
        awaitEvent { it.optLong("revision") == 1L && it.optString("state") == "paused" && it.optDouble("position") >= 399.0 }
        PlatformAudio.command(JSONObject().put("op","seek").put("token",token).put("revision",2).put("position",30.0).toString())
        awaitEvent { it.optLong("revision") == 2L && it.optString("state") == "paused" && it.optDouble("position") in 29.0..31.0 }
        command("resume",3)
        awaitEvent { it.optLong("revision") == 3L && it.optString("state") == "playing" && it.optDouble("position") > 31.0 }
    }

    @Test fun tenMinuteTrackPlaysAllTheWayAtFourTimesSpeed() {
        server.data = InstrumentationRegistry.getInstrumentation().context.assets.open("long-tone.m4a").use { it.readBytes() }
        command("play", 0)
        val ready = awaitEvent { it.optBoolean("ready") && it.optDouble("duration") > 590.0 }
        assertTrue(ready.optDouble("duration") < 610.0)
        // Test-only reflection changes the actual ExoPlayer clock rate, not its
        // source/position/state. No production API or fake completed events.
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            val field = PlatformAudio::class.java.getDeclaredField("player")
            field.isAccessible = true
            val audio = field.get(PlatformAudio) as androidx.media3.exoplayer.ExoPlayer
            audio.setPlaybackSpeed(4.0f)
        }
        awaitEvent(100000) { it.optString("state") == "playing" && it.optDouble("position") > 300.0 }
        val ended = awaitEvent(100000) { it.optString("state") == "ended" }
        assertTrue(ended.optDouble("position") > 590.0)
        assertTrue("Whole long file must actually cross the fixture socket", server.mediaBytes.get() >= server.data.size)
    }

    @Test fun tenSecondSourceGapDoesNotTripTheDefaultEightSecondReadTimeout() {
        server.gapMillis = 10000
        command("play", 0)
        awaitEvent(30000) { it.optString("state") == "playing" && it.optDouble("position") >= 3.0 }
        awaitEvent(20000) { it.optString("state") == "ended" }
    }

    private class FixtureServer(@Volatile var data: ByteArray) : Closeable {
        val events = ArrayBlockingQueue<JSONObject>(256)
        val mediaRequests = AtomicInteger()
        val mediaBytes = AtomicInteger()
        @Volatile var gapMillis = 0L
        @Volatile var rejectMedia = false
        private val socket = ServerSocket(BuildConfig.API_PORT, 16, InetAddress.getByName("127.0.0.1"))
        private val workers = Executors.newFixedThreadPool(4)
        private val accepts = Thread {
            while (!socket.isClosed) {
                try { val connection = socket.accept(); workers.execute { connection.use { handle(it) } } }
                catch (_: Exception) { if (!socket.isClosed) throw AssertionError("Fixture accept failed") }
            }
        }
        fun start() = accepts.start()
        private fun handle(connection: Socket) {
            connection.soTimeout = 3000
            val input = connection.getInputStream().buffered()
            val header = ArrayList<Byte>()
            while (header.size < 16384) {
                val next = input.read(); if (next < 0) return
                header.add(next.toByte())
                if (header.size >= 4 && header.takeLast(4) == listOf(13.toByte(), 10.toByte(), 13.toByte(), 10.toByte())) break
            }
            val lines = header.toByteArray().toString(Charsets.US_ASCII).split("\r\n")
            val first = lines[0].split(" ")
            if (first.size < 2) return
            val headers = lines.drop(1).mapNotNull { line ->
                val index = line.indexOf(':'); if (index < 0) null else line.substring(0, index).lowercase() to line.substring(index + 1).trim()
            }.toMap()
            val length = headers["content-length"]?.toIntOrNull() ?: 0
            require(length in 0..16384)
            val body = ByteArray(length)
            var offset = 0
            while (offset < length) { val n = input.read(body, offset, length - offset); if (n < 0) return; offset += n }
            when (first[1]) {
                "/api/status" -> reply(connection, 200, JSONObject().put("instance_id", MediaApi.instanceId)
                    .put("audio_engine", BuildConfig.AUDIO_ENGINE).put("api_port", BuildConfig.API_PORT).toString().toByteArray(), "application/json")
                "/api/platform/event" -> { events.offer(JSONObject(body.toString(Charsets.UTF_8))); reply(connection, 200, "{}".toByteArray(), "application/json") }
                "/api/platform/media/fixture" -> {
                    mediaRequests.incrementAndGet()
                    if (rejectMedia) { reply(connection, 403, ByteArray(0)); return }
                    val range = headers["range"]?.removePrefix("bytes=")?.split('-')
                    val start = range?.getOrNull(0)?.toIntOrNull() ?: 0
                    val end = (range?.getOrNull(1)?.toIntOrNull() ?: data.lastIndex).coerceAtMost(data.lastIndex)
                    if (start !in data.indices || end < start) { reply(connection, 416, ByteArray(0)); return }
                    val extra = if (range != null) "Content-Range: bytes $start-$end/${data.size}\r\n" else ""
                    reply(connection, if (range != null) 206 else 200, data.copyOfRange(start, end + 1), "audio/mp4", extra, first[0] != "HEAD")
                }
                else -> reply(connection, 404, ByteArray(0))
            }
        }
        private fun reply(socket: Socket, status: Int, body: ByteArray, type: String = "text/plain", extra: String = "", sendBody: Boolean = true) {
            val output = socket.getOutputStream()
            output.write(("HTTP/1.1 $status Fixture\r\nContent-Type: $type\r\nContent-Length: ${body.size}\r\nAccept-Ranges: bytes\r\n${extra}Connection: close\r\n\r\n").toByteArray())
            if (sendBody) {
                if (type == "audio/mp4" && gapMillis > 0 && body.size > 16384) {
                    output.write(body, 0, 16384)
                    output.flush()
                    Thread.sleep(gapMillis)
                    output.write(body, 16384, body.size - 16384)
                } else output.write(body)
                if (type == "audio/mp4") mediaBytes.addAndGet(body.size)
            }
            output.flush()
        }
        override fun close() { socket.close(); accepts.join(1000); workers.shutdownNow() }
    }
}
