package com.afzal.ayshamusic

import android.annotation.SuppressLint
import android.webkit.ConsoleMessage
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayInputStream
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

/** Isolated research fixture: copied into androidTest ONLY by its own workflow.
 * No Rust/JNI, provider access, native JS bridge or music playback is involved.
 */
@RunWith(AndroidJUnit4::class)
class BrowserExtractorFixtureTest {
    @SuppressLint("SetJavaScriptEnabled")
    @Test fun pinnedMitLibraryParsesOfflineInActualAndroidWebView() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val script = instrumentation.context.assets.open("extractor/fixture.js").use { it.readBytes() }
        val results = ArrayBlockingQueue<String>(8)
        val external = AtomicInteger()
        val consoleErrors = AtomicInteger()
        val firstError = ArrayBlockingQueue<String>(1)
        var webVersion = "unknown"
        lateinit var web: WebView
        instrumentation.runOnMainSync {
            web = WebView(instrumentation.targetContext)
            webVersion = WebView.getCurrentWebViewPackage()?.versionName ?: "unknown"
            web.settings.apply {
                javaScriptEnabled = true
                allowFileAccess = false
                allowContentAccess = false
                blockNetworkLoads = true
                domStorageEnabled = false
                mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            }
            web.webViewClient = object : WebViewClient() {
                override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse {
                    val uri = request.url
                    if (request.method == "GET" && uri.scheme == "https" &&
                        uri.host == "aysha-extractor.test" && uri.port == -1 && uri.query == null) {
                        if (uri.path == "/") return WebResourceResponse("text/html", "UTF-8",
                            ByteArrayInputStream(("<!doctype html><title>Offline fixture</title>" +
                                "<script src='/fixture.js'></script>").toByteArray()))
                        if (uri.path == "/fixture.js") return WebResourceResponse("application/javascript", "UTF-8", ByteArrayInputStream(script))
                    }
                    external.incrementAndGet()
                    return WebResourceResponse("text/plain", "UTF-8", 403, "Blocked", emptyMap(), ByteArrayInputStream(ByteArray(0)))
                }
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    external.incrementAndGet()
                    return true
                }
            }
            web.webChromeClient = object : WebChromeClient() {
                override fun onConsoleMessage(message: ConsoleMessage): Boolean {
                    val prefix = "AYSHA_FIXTURE_RESULT "
                    if (message.message().startsWith(prefix)) results.offer(message.message().removePrefix(prefix))
                    else if (message.messageLevel() == ConsoleMessage.MessageLevel.ERROR) {
                        consoleErrors.incrementAndGet()
                        // Fixture-only local script; no real provider body exists.
                        firstError.offer(message.message().take(300))
                    }
                    // Do not forward arbitrary script/provider errors into test logs.
                    return true
                }
            }
            web.loadUrl("https://aysha-extractor.test/")
        }
        try {
            val raw = results.poll(45, TimeUnit.SECONDS)
            assertNotNull("Offline WebView fixture deadline; version=$webVersion; errors=${consoleErrors.get()}; first=${firstError.peek()}", raw)
            val result = JSONObject(raw!!)
            assertEquals("Offline fixture result: $raw; WebView=$webVersion", "pass", result.getString("status"))
            assertEquals("browser", result.getString("runtime"))
            assertEquals(140, result.getInt("itag"))
            assertEquals(1, result.getInt("fixture_requests"))
            assertFalse(result.getBoolean("media_validated"))
            assertFalse(result.getBoolean("playback_tested"))
            assertEquals("[\"provider_login_or_challenge\",\"video_id_mismatch\",\"interpreter_required\"]",
                result.getJSONArray("negative").toString())
            assertEquals("No network/navigation or unexpected asset request", 0, external.get())
            assertEquals("No hidden script error", 0, consoleErrors.get())
        } finally {
            instrumentation.runOnMainSync { web.stopLoading(); web.destroy() }
        }
    }
}
