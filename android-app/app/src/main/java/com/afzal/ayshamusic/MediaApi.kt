package com.afzal.ayshamusic

import android.util.Log
import java.net.HttpURLConnection
import java.net.URL

/**
 * Minimal localhost HTTP client for the Rust server.
 * Uses java.net.HttpURLConnection to avoid bundling OkHttp/Retrofit.
 */
object MediaApi {
    private const val TAG = "MediaApi"
    val baseUrl = "http://127.0.0.1:${BuildConfig.API_PORT}"
    val instanceId: String = java.util.UUID.randomUUID().toString()
    @Volatile private var verified = false

    fun isBackendVerified(): Boolean = verified

    fun verifyBackend(): Boolean {
        return try {
            val status = org.json.JSONObject(get("/api/status") ?: run { verified = false; return false })
            (status.optString("instance_id") == instanceId &&
                status.optString("audio_engine") == BuildConfig.AUDIO_ENGINE &&
                status.optInt("api_port") == BuildConfig.API_PORT).also { verified = it }
        } catch (_: Exception) { verified = false; false }
    }
    private const val TIMEOUT_MS = 1500
    private val posts = java.util.concurrent.Executors.newSingleThreadExecutor()
    // Playback-ready events must reach Rust while a control request is waiting
    // for preparation; putting both on one executor causes a circular wait.
    private val events = java.util.concurrent.Executors.newSingleThreadExecutor()

    fun post(path: String, body: String? = null) {
        if (!verified) return
        val event = path == "/api/platform/event"
        val executor = if (event) events else posts
        executor.execute {
            var connection: HttpURLConnection? = null
            try {
                val url = URL(baseUrl + path)
                connection = (url.openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = TIMEOUT_MS
                    readTimeout = if (event) TIMEOUT_MS else 60000
                    setRequestProperty("Content-Type", "application/json")
                    if (body != null) {
                        doOutput = true
                        outputStream.use { it.write(body.toByteArray()) }
                    } else {
                        doOutput = true
                        outputStream.use { it.write(ByteArray(0)) }
                    }
                }
                val code = connection.responseCode
                if (code !in 200..299) Log.w(TAG, "POST $path -> HTTP $code")
            } catch (e: Exception) {
                Log.w(TAG, "POST $path failed: ${e.message}")
            } finally {
                connection?.disconnect()
            }
        }
    }

    fun get(path: String): String? {
        if (!verified && path != "/api/status") return null
        var connection: HttpURLConnection? = null
        return try {
            val url = URL(baseUrl + path)
            connection = (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "GET"
                connectTimeout = TIMEOUT_MS
                readTimeout = TIMEOUT_MS
            }
            if (connection.responseCode !in 200..299) return null
            connection.inputStream.bufferedReader().use { it.readText() }
        } catch (e: Exception) {
            Log.w(TAG, "GET $path failed: ${e.message}")
            null
        } finally {
            connection?.disconnect()
        }
    }
}