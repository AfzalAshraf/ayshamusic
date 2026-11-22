package com.afzal.ayshamusic

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

/** Foreground/open-app checks. Never silently install, trust a new signer, or erase data. */
object AppUpdater {
    private lateinit var app:Context
    private val main=Handler(Looper.getMainLooper())
    private val worker=Executors.newSingleThreadExecutor()
    @Volatile var listener:(()->Unit)?=null
    @Volatile private var busy=false
    @Volatile private var message="No signed release checked yet. Existing preview signing compatibility is unknown."
    @Volatile private var release:UpdatePolicy.Release?=null
    @Volatile private var ready:File?=null
    fun initialize(context:Context){app=context.applicationContext}
    private fun prefs()=app.getSharedPreferences("app_updates",Context.MODE_PRIVATE)
    fun enabled()=prefs().getBoolean("wifi_auto",true)
    fun toggle(){prefs().edit().putBoolean("wifi_auto",!enabled()).apply();notifyChanged()}
    fun wifi():Boolean {
        val cm=app.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network=cm.activeNetwork ?: return false
        return !cm.isActiveNetworkMetered && cm.getNetworkCapabilities(network)?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)==true
    }
    fun state()=JSONObject().put("version",BuildConfig.VERSION_NAME).put("auto",enabled()).put("busy",busy).put("message",message)
        .put("available",release!=null).put("ready",ready?.isFile==true).put("latest",release?.version ?: "")
    private fun notifyChanged(){main.post{listener?.invoke()}}
    private fun report(value:String){message=value;notifyChanged()}
    fun automatic(){
        if(!enabled() || !wifi() || System.currentTimeMillis()-prefs().getLong("last_check",0)<24*60*60*1000L)return
        check(true)
    }
    @Synchronized fun check(automatic:Boolean=false){
        if(busy)return
        if(automatic && (!enabled() || !wifi()))return
        busy=true;report("Checking the official signed release…")
        worker.execute{
            try{
                if(automatic)prefs().edit().putLong("last_check",System.currentTimeMillis()).apply()
                val conn=open(UpdatePolicy.MANIFEST)
                val bytes=try { require(conn.contentLengthLong<=64*1024);conn.inputStream.use{it.readBytesBounded(64*1024)} }finally{conn.disconnect()}
                val candidate=UpdatePolicy.parse(JSONObject(String(bytes,Charsets.UTF_8)))
                if(candidate.code<=UpdatePolicy.version(UpdatePolicy.info(app))){release=null;ready=null;report("You have the latest compatible published version.")}
                else if(candidate.certificate!=UpdatePolicy.installedCertificate(app)){release=null;ready=null;report("Update uses a different signing key. Keep this installation; do not uninstall or clear data.")}
                else if(!Build.SUPPORTED_ABIS.contains("arm64-v8a")){release=null;ready=null;report("This update requires an ARM64 phone.")}
                else {
                    if(release?.code!=candidate.code)ready=null
                    release=candidate
                    report("Version ${candidate.version} is available.")
                    if(automatic && enabled() && wifi())downloadFile(candidate,true)
                }
            }catch(_:Exception){report("No verified signed update is available right now. Release signing may still need configuration. Try again later; keep this installation.")}
            finally{busy=false;notifyChanged()}
        }
    }
    @Synchronized fun download(){
        val candidate=release ?: return
        if(busy)return
        busy=true;worker.execute{try{downloadFile(candidate,false)}catch(_:Exception){report("Update download or verification failed. Nothing was installed. Keep your current app.")}finally{busy=false;notifyChanged()}}
    }
    private fun downloadFile(candidate:UpdatePolicy.Release,automatic:Boolean){
        val dir=File(app.cacheDir,"updates").apply{mkdirs()}
        val part=File(dir,"candidate.part");val apk=File(dir,"candidate.apk")
        ready=null;part.delete();apk.delete();report("Downloading ${candidate.version}…")
        try{
            val conn=open(candidate.url)
            val start=SystemClock.elapsedRealtime()
            try{
                require(conn.contentLengthLong==-1L || conn.contentLengthLong==candidate.size)
                var count=0L;var lastProgress=0
                conn.inputStream.use{input->part.outputStream().use{out->
                    val buffer=ByteArray(64*1024)
                    while(true){
                        require(SystemClock.elapsedRealtime()-start<10*60*1000)
                        if(automatic)require(enabled() && wifi())
                        val n=input.read(buffer);if(n<0)break
                        count+=n;require(count<=candidate.size && count<=UpdatePolicy.MAX_APK)
                        out.write(buffer,0,n)
                        val percent=(count*100/candidate.size).toInt()
                        if(percent>=lastProgress+10){lastProgress=percent;report("Downloading update · $percent%")}
                    }
                }}
                require(count==candidate.size)
            }finally{conn.disconnect()}
            UpdatePolicy.verifyApk(app,part,candidate)
            require(part.renameTo(apk));ready=apk;report("Verified update ready. Tap Install update; Android will ask you to confirm.")
        }finally{part.delete()}
    }
    fun install(activity:Activity){
        val file=ready ?: return;val candidate=release ?: return
        if(!file.isFile)return
        try{
            // Revalidate on every explicit installation request, including after
            // returning from unknown-source settings. Never auto-open installer.
            UpdatePolicy.verifyApk(app,file,candidate)
            if(!app.packageManager.canRequestPackageInstalls()){
                report("Allow updates from Aysha in Android settings, then return and tap Install update again.")
                activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:${app.packageName}")));return
            }
            val uri=FileProvider.getUriForFile(app,app.packageName+".updates",file)
            activity.startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        }catch(_:Exception){ready=null;report("Android could not open this update safely. Keep your installed app; do not uninstall.")}
    }
    private fun java.io.InputStream.readBytesBounded(limit:Int):ByteArray {
        val out=java.io.ByteArrayOutputStream();val buffer=ByteArray(4096)
        while(true){val n=read(buffer);if(n<0)break;require(out.size()+n<=limit);out.write(buffer,0,n)};return out.toByteArray()
    }
    private fun open(url:String):HttpURLConnection {
        var target=url
        repeat(6){
            require(UpdatePolicy.allowedConnection(target))
            val conn=(URL(target).openConnection() as HttpURLConnection).apply{instanceFollowRedirects=false;connectTimeout=15000;readTimeout=20000;setRequestProperty("User-Agent","AyshaMusic-Updates");setRequestProperty("Accept","application/octet-stream");useCaches=false}
            val status=conn.responseCode
            if(status in listOf(301,302,303,307,308)){
                val location=conn.getHeaderField("Location") ?: run{conn.disconnect();error("Redirect missing")}
                target=URL(URL(target),location).toString();conn.disconnect()
            }else{if(status!=200){conn.disconnect();error("Release unavailable")};return conn}
        }
        error("Too many redirects")
    }
}
