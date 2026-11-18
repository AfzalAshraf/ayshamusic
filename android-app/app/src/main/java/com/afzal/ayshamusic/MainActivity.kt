package com.afzal.ayshamusic

import android.annotation.SuppressLint
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.KeyEvent
import android.view.View
import android.webkit.CookieManager
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

/**
 * MainActivity - Fullscreen WebView pointing to the verified variant-specific loopback server.
 * 
 * Key properties:
 * - No Jetpack Compose, no Navigation, no Fragments
 * - Predictive back gesture support (Android 14+)
 * - Cleartext traffic allowed for localhost
 * - JavaScript enabled, DOM storage enabled
 * - Hardware acceleration for smooth scrolling
 */
class MainActivity : AppCompatActivity() {
    private companion object {
        private const val TAG = "MainActivity"
        private val SERVER_URL get() = MediaApi.baseUrl
    }

    @Volatile private var destroyed = false
    private var webView: WebView? = null
    private var pendingSharedId: String? = null
    private var localPageReady = false
    private var backCallbackRegistered = false
    private var backCallback: android.window.OnBackInvokedCallback? = null

    @SuppressLint("SetJavaScriptEnabled", "JavascriptInterface")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Edge-to-edge display for modern Android
        WindowCompat.setDecorFitsSystemWindows(window, false)
        
        // Start Rust server FIRST before setting up WebView
        RustBridge.startServer(this)

        // Foreground media service (notification + MediaSession + WakeLock)
        MusicForegroundService.start(this)

        // Create WebView programmatically (no layout XML needed)
        webView = WebView(this).apply {
            id = View.generateViewId()
            layoutParams = android.view.ViewGroup.LayoutParams(
                android.view.ViewGroup.LayoutParams.MATCH_PARENT,
                android.view.ViewGroup.LayoutParams.MATCH_PARENT
            )
        }
        setContentView(webView!!)

        pendingSharedId = savedInstanceState?.getString("shared_video")
        if (savedInstanceState == null) acceptShare(intent)
        setupWebView()
        loadServerUrl()
        if (Build.VERSION.SDK_INT >= 33 && androidx.core.content.ContextCompat.checkSelfPermission(this, android.Manifest.permission.POST_NOTIFICATIONS) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            androidx.core.app.ActivityCompat.requestPermissions(this, arrayOf(android.Manifest.permission.POST_NOTIFICATIONS), 100)
        }
        
        // Register predictive back callback (Android 14+)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            registerPredictiveBack()
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun setupWebView() {
        webView!!.apply {
            val settings: WebSettings = this.settings.apply {
                // JavaScript & DOM
                javaScriptEnabled = true
                domStorageEnabled = true
                databaseEnabled = true
                allowFileAccess = false
                allowContentAccess = false
                allowFileAccessFromFileURLs = false
                allowUniversalAccessFromFileURLs = false
                
                // Caching - no cache for dynamic content
                cacheMode = WebSettings.LOAD_NO_CACHE
                
                // Viewport & zoom
                useWideViewPort = true
                loadWithOverviewMode = true
                setSupportZoom(false)
                builtInZoomControls = false
                displayZoomControls = false
                
                // Media playback
                mediaPlaybackRequiresUserGesture = false
                
                // Mixed content - allow HTTP on localhost
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                    mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                }
                
                // User agent
                userAgentString = "${settings.userAgentString} AyshaMusic/1.0"
            }

            // WebViewClient for navigation handling
            webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    val url = request.url.toString()
                    if (request.url.scheme == "aysha" && request.url.host == "retry") {
                        loadServerUrl()
                        return true
                    }
                    // Keep localhost navigation in WebView
                    if (request.url.scheme == "http" && request.url.host == "127.0.0.1" && request.url.port == BuildConfig.API_PORT) {
                        return false
                    }
                    // Open external links in browser
                    if (url.startsWith("http://") || url.startsWith("https://")) {
                        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
                        startActivity(intent)
                        return true
                    }
                    return true
                }

                override fun onPageFinished(view: WebView, url: String) {
                    val uri = Uri.parse(url)
                    localPageReady = uri.scheme == "http" && uri.host == "127.0.0.1" && uri.port == BuildConfig.API_PORT && uri.path == "/" && MediaApi.isBackendVerified()
                    if (localPageReady) deliverShare()
                }

                override fun onReceivedError(view: WebView, request: WebResourceRequest?, error: android.webkit.WebResourceError) {
                    Log.e(TAG, "WebView error: ${error.description} (${error.errorCode})")
                    if (request?.isForMainFrame == true && request.url.host == "127.0.0.1") {
                        view.post { if (!destroyed) showLocalStartupError() }
                    }
                }
            }

            // WebChromeClient for console logs, fullscreen video, etc.
            webChromeClient = object : WebChromeClient() {
                override fun onConsoleMessage(consoleMessage: android.webkit.ConsoleMessage): Boolean {
                    // Provider/session errors may contain private data; no raw console forwarding in release.
                    if (BuildConfig.DEBUG) Log.d("WebViewConsole", "${consoleMessage.messageLevel()}: local UI message")
                    return true
                }

                override fun onPermissionRequest(request: android.webkit.PermissionRequest) {
                    // Grant media permissions for audio playback
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                        request.deny()
                    }
                }
            }

            // Hardware acceleration for smooth rendering
            setLayerType(View.LAYER_TYPE_HARDWARE, null)
            
            // Scrollbar style
            isScrollbarFadingEnabled = true
            verticalScrollbarPosition = View.SCROLLBARS_INSIDE_OVERLAY
        }

        // Accept cookies for session persistence
        CookieManager.getInstance().setAcceptCookie(true)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            CookieManager.getInstance().setAcceptThirdPartyCookies(webView!!, false)
        }
    }

    private fun loadServerUrl() {
        localPageReady = false
        webView!!.loadData("<html><body style='background:#100d13;color:#fff;font-family:sans-serif;padding:32px'>Starting Aysha Music…</body></html>", "text/html", "UTF-8")
        Thread {
            var ready = false
            for (attempt in 1..30) {
                if (destroyed) return@Thread
                if (MediaApi.verifyBackend()) { ready = true; break }
                Thread.sleep(200)
            }
            runOnUiThread {
                if (!destroyed) {
                    if (ready) webView?.loadUrl(SERVER_URL)
                    else showLocalStartupError()
                }
            }
        }.start()
    }

    private fun showLocalStartupError() {
        webView?.loadData("<html><meta name='viewport' content='width=device-width,initial-scale=1'><body style='background:#100d13;color:white;font-family:sans-serif;padding:32px'><h2>Local player could not open</h2><p>Aysha runs inside this APK. No external Aysha server is required.</p><p><a style='color:#f2bdc8' href='aysha://retry'>Retry local player</a></p></body></html>", "text/html", "UTF-8")
    }

    // Predictive back gesture (Android 14+)
    private fun registerPredictiveBack() {
        if (backCallbackRegistered) return
        
        val callback = object : android.window.OnBackInvokedCallback {
            override fun onBackInvoked() {
                handleBackPress()
            }
        }
        backCallback = callback
        onBackInvokedDispatcher.registerOnBackInvokedCallback(
            android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            callback
        )
        backCallbackRegistered = true
    }

    private fun handleBackPress() {
        webView?.let { wv ->
            if (wv.canGoBack()) {
                wv.goBack()
            } else {
                // At root - minimize app instead of exiting (music keeps playing)
                moveTaskToBack(true)
            }
        }
    }

    // Hardware back button (pre-Android 14)
    override fun onKeyDown(keyCode: Int, event: KeyEvent?): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            handleBackPress()
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    private fun acceptShare(incoming: Intent?) {
        if (incoming?.action != Intent.ACTION_SEND || incoming.type != "text/plain") return
        val id = SharedYoutubeLink.videoId(incoming.getStringExtra(Intent.EXTRA_TEXT))
        if (id == null) android.widget.Toast.makeText(this, "Share one supported YouTube video link", android.widget.Toast.LENGTH_LONG).show()
        else pendingSharedId = id
    }

    private fun deliverShare() {
        if (!localPageReady || !MediaApi.isBackendVerified()) return
        val id = pendingSharedId ?: return
        pendingSharedId = null
        webView?.evaluateJavascript("receiveSharedVideo(" + org.json.JSONObject.quote(id) + ")", null)
    }

    override fun onSaveInstanceState(outState: Bundle) {
        pendingSharedId?.let { outState.putString("shared_video", it) }
        super.onSaveInstanceState(outState)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        acceptShare(intent)
        deliverShare()
    }

    override fun onDestroy() {
        destroyed = true
        // Clean up WebView
        webView?.apply {
            stopLoading()
            clearHistory()
            clearCache(true)
            destroy()
        }
        webView = null
        
        // Unregister predictive back
        if (backCallbackRegistered && Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            backCallback?.let { callback ->
                onBackInvokedDispatcher.unregisterOnBackInvokedCallback(callback)
            }
            backCallback = null
            backCallbackRegistered = false
        }
        
        super.onDestroy()
    }
}