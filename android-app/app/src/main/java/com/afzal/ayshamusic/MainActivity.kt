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
 * MainActivity - Ultra-minimal fullscreen WebView pointing to localhost:5050.
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
        private const val SERVER_URL = "http://127.0.0.1:5050"
    }

    @Volatile private var destroyed = false
    private var webView: WebView? = null
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

        setupWebView()
        loadServerUrl()
        
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
                    mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
                }
                
                // User agent
                userAgentString = "${settings.userAgentString} AyshaMusic/1.0"
            }

            // WebViewClient for navigation handling
            webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    val url = request.url.toString()
                    // Keep localhost navigation in WebView
                    if (url.startsWith("http://127.0.0.1:5050") || url.startsWith("https://127.0.0.1:5050")) {
                        return false
                    }
                    // Open external links in browser
                    if (url.startsWith("http://") || url.startsWith("https://")) {
                        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
                        startActivity(intent)
                        return true
                    }
                    return false
                }

                override fun onReceivedError(view: WebView, request: WebResourceRequest?, error: android.webkit.WebResourceError) {
                    Log.e(TAG, "WebView error: ${error.description} (${error.errorCode})")
                }
            }

            // WebChromeClient for console logs, fullscreen video, etc.
            webChromeClient = object : WebChromeClient() {
                override fun onConsoleMessage(consoleMessage: android.webkit.ConsoleMessage): Boolean {
                    Log.d("WebViewConsole", "[${consoleMessage.messageLevel()}] ${consoleMessage.message()} (${consoleMessage.sourceId()}:${consoleMessage.lineNumber()})")
                    return true
                }

                override fun onPermissionRequest(request: android.webkit.PermissionRequest) {
                    // Grant media permissions for audio playback
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                        request.grant(request.resources)
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
            CookieManager.getInstance().setAcceptThirdPartyCookies(webView!!, true)
        }
    }

    private fun loadServerUrl() {
        webView!!.loadData("<html><body style='background:#100d13;color:#fff;font-family:sans-serif;padding:32px'>Starting Aysha Music…</body></html>", "text/html", "UTF-8")
        Thread {
            var ready = false
            for (attempt in 1..30) {
                if (destroyed) return@Thread
                if (MediaApi.get("/api/status") != null) { ready = true; break }
                Thread.sleep(200)
            }
            runOnUiThread {
                if (!destroyed) {
                    if (ready) webView?.loadUrl(SERVER_URL)
                    else webView?.loadData("<html><body style='background:#100d13;color:white;padding:32px'>Aysha could not start. Close and reopen the app to retry.</body></html>", "text/html", "UTF-8")
                }
            }
        }.start()
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

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        // Handle deep links if needed
        intent.data?.let { uri ->
            if (uri.toString().startsWith("http://127.0.0.1:5050")) {
                webView?.loadUrl(uri.toString())
            }
        }
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