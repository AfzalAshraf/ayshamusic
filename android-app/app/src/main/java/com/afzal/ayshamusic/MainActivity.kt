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
            // WebView defaults to white before the local page first paints.
            // Set the background before attaching it to the window.
            setBackgroundColor(android.graphics.Color.rgb(3, 3, 3))
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
        // Existing write and notification permissions; music access is requested from Library.
        val permissions=mutableListOf<String>()
        if(Build.VERSION.SDK_INT<=28 && androidx.core.content.ContextCompat.checkSelfPermission(this,android.Manifest.permission.WRITE_EXTERNAL_STORAGE)!=android.content.pm.PackageManager.PERMISSION_GRANTED)permissions.add(android.Manifest.permission.WRITE_EXTERNAL_STORAGE)
        if(Build.VERSION.SDK_INT>=33 && androidx.core.content.ContextCompat.checkSelfPermission(this,android.Manifest.permission.POST_NOTIFICATIONS)!=android.content.pm.PackageManager.PERMISSION_GRANTED)permissions.add(android.Manifest.permission.POST_NOTIFICATIONS)
        if(permissions.isNotEmpty())androidx.core.app.ActivityCompat.requestPermissions(this,permissions.toTypedArray(),101)

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
                    if (request.url.scheme == "aysha" && request.url.host == "app") {
                        if (request.isForMainFrame && trustedSettingsPage(view) && request.url.query == null && request.url.fragment == null) handleAppAction(request.url.path.orEmpty())
                        return true
                    }
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
                    if (localPageReady) {
                        deliverShare()
                        publishAppSettings()
                    }
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

    private fun trustedSettingsPage(view:WebView):Boolean {
        val uri=Uri.parse(view.url ?: return false)
        return localPageReady && MediaApi.isBackendVerified() && uri.scheme=="http" && uri.host=="127.0.0.1" && uri.port==BuildConfig.API_PORT && uri.path=="/" && uri.query==null
    }

    private fun publishAppSettings() {
        val view=webView ?: return
        if(!trustedSettingsPage(view))return
        val value=org.json.JSONObject().put("icon",AppIcons.title(this)).put("legacyStorage",Build.VERSION.SDK_INT <= 28).put("musicPermissionMissing",!LocalMusic.hasReadAccess(this))
        view.evaluateJavascript("if(window.renderNativeAppSettings)renderNativeAppSettings("+value.toString()+")",null)
    }

    private fun handleAppAction(path:String) {
        when(path){
            "/icons" -> showIconPicker()
            "/music-permission" -> {
                val permission=if(Build.VERSION.SDK_INT>=33) android.Manifest.permission.READ_MEDIA_AUDIO else android.Manifest.permission.READ_EXTERNAL_STORAGE
                if(androidx.core.content.ContextCompat.checkSelfPermission(this,permission)!=android.content.pm.PackageManager.PERMISSION_GRANTED) androidx.core.app.ActivityCompat.requestPermissions(this,arrayOf(permission),102)
                else startActivity(Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS,Uri.parse("package:$packageName")))
            }
            "/storage-settings" -> startActivity(Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS,Uri.parse("package:$packageName")))

        }
    }

    private fun showIconPicker() {
        val selected=AppIcons.choices.indexOfFirst{it.id==AppIcons.current(this)}
        val adapter=object:android.widget.ArrayAdapter<String>(this,android.R.layout.simple_list_item_single_choice,AppIcons.choices.map{it.title}) {
            override fun getView(position:Int,convertView:View?,parent:android.view.ViewGroup):View {
                val text=super.getView(position,convertView,parent) as android.widget.TextView
                val drawable=androidx.core.content.ContextCompat.getDrawable(this@MainActivity,AppIcons.choices[position].preview)
                val size=(52*resources.displayMetrics.density).toInt();drawable?.setBounds(0,0,size,size)
                text.setCompoundDrawables(drawable,null,null,null);text.compoundDrawablePadding=(16*resources.displayMetrics.density).toInt()
                text.minHeight=(76*resources.displayMetrics.density).toInt();return text
            }
        }
        android.app.AlertDialog.Builder(this).setTitle("Choose Aysha’s app icon")
            .setSingleChoiceItems(adapter,selected){dialog,which->
                try{AppIcons.select(this,AppIcons.choices[which].id);publishAppSettings();android.widget.Toast.makeText(this,"Icon changed. Your launcher may take a moment to refresh.",android.widget.Toast.LENGTH_LONG).show()}
                catch(_:Exception){android.widget.Toast.makeText(this,"This launcher could not change the icon. No app data was changed.",android.widget.Toast.LENGTH_LONG).show()}
                dialog.dismiss()
            }.setNegativeButton("Close",null).show()
    }

    override fun onResume() {
        super.onResume()
        if(localPageReady)publishAppSettings()
    }

    private fun acceptShare(incoming: Intent?) {
        if (incoming?.action != Intent.ACTION_SEND || incoming.type != "text/plain") return
        val text = incoming.getStringExtra(Intent.EXTRA_TEXT)
        val id = if (text == null || text.length > 4096) null else try {
            RustBridge.sharedVideoId(text)
        } catch (_: UnsatisfiedLinkError) { null }
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