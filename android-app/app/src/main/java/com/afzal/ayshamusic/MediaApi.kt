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
    private const val BASE = "http://127.0.0.1:5050"
    private const val TIMEOUT_MS = 1500

    fun post(path: String, body: String? = null) {
        Thread {
            var connection: HttpURLConnection? = null
            try {
                val url = URL(BASE + path)
                connection = (url.openConnection() as HttpURLConnection).apply {
                    requestMethod = "POST"
                    connectTimeout = TIMEOUT_MS
                    readTimeout = TIMEOUT_MS
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
        }.start()
    }

    fun get(path: String): String? {
        var connection: HttpURLConnection? = null
        return try {
            val url = URL(BASE + path)
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