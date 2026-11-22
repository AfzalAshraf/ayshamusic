package com.afzal.ayshamusic

import android.content.Context
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.os.Build
import org.json.JSONObject
import java.io.File
import java.net.URI
import java.security.MessageDigest

object UpdatePolicy {
    const val MANIFEST="https://github.com/AfzalAshraf/ayshamusic/releases/latest/download/update.json"
    const val MAX_APK=15L*1024*1024
    data class Release(val code:Long,val version:String,val size:Long,val sha:String,val certificate:String,val url:String)
    fun parse(json:JSONObject):Release {
        require(json.getInt("schema")==1 && json.getString("package_id")=="com.afzal.ayshamusic.androidaudio10")
        require(json.getString("abi")=="arm64-v8a" && json.getInt("min_sdk")==26)
        val code=json.getLong("version_code");require(code in 1..2_100_000_000L)
        val name=json.getString("version_name");require(name.length in 1..80 && name.none{it.isISOControl()})
        val size=json.getLong("size");require(size in 1..MAX_APK)
        val sha=json.getString("sha256").lowercase();val cert=json.getString("certificate_sha256").lowercase()
        require(sha.matches(Regex("[a-f0-9]{64}")) && cert.matches(Regex("[a-f0-9]{64}")))
        val url=json.getString("url")
        require(url=="https://github.com/AfzalAshraf/ayshamusic/releases/download/android-v$code/AyshaMusic-arm64.apk")
        return Release(code,name,size,sha,cert,url)
    }
    fun allowedConnection(raw:String):Boolean=try {
        val u=URI(raw)
        u.scheme=="https" && u.rawUserInfo==null && u.rawFragment==null && (u.port==-1 || u.port==443) &&
            u.host in setOf("github.com","release-assets.githubusercontent.com","objects.githubusercontent.com")
    }catch(_:Exception){false}
    @Suppress("DEPRECATION") fun info(context:Context):PackageInfo=context.packageManager.getPackageInfo(context.packageName,if(Build.VERSION.SDK_INT>=28) PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES)
    @Suppress("DEPRECATION") fun version(info:PackageInfo):Long=if(Build.VERSION.SDK_INT>=28)info.longVersionCode else info.versionCode.toLong()
    @Suppress("DEPRECATION") fun certificates(info:PackageInfo):Set<String> {
        val signatures=if(Build.VERSION.SDK_INT>=28)info.signingInfo?.apkContentsSigners else info.signatures
        return signatures?.map{hex(MessageDigest.getInstance("SHA-256").digest(it.toByteArray()))}?.toSet() ?: emptySet()
    }
    fun installedCertificate(context:Context):String {
        val set=certificates(info(context));require(set.size==1){"Unsupported installed signing identity"};return set.single()
    }
    fun hex(bytes:ByteArray)=bytes.joinToString(""){"%02x".format(it.toInt() and 255)}
    @Suppress("DEPRECATION") fun verifyApk(context:Context,file:File,release:Release) {
        require(file.length()==release.size)
        val digest=MessageDigest.getInstance("SHA-256");file.inputStream().use { input -> val buffer=ByteArray(64*1024);while(true){val count=input.read(buffer);if(count<0)break;digest.update(buffer,0,count)} }
        require(hex(digest.digest())==release.sha){"Checksum mismatch"}
        val pm=context.packageManager
        val archive=pm.getPackageArchiveInfo(file.absolutePath,if(Build.VERSION.SDK_INT>=28)PackageManager.GET_SIGNING_CERTIFICATES else PackageManager.GET_SIGNATURES) ?: error("Invalid APK")
        val current=info(context)
        require(archive.packageName==context.packageName && version(archive)==release.code && release.code>version(current)){"Wrong package or version"}
        val installed=certificates(current);val incoming=certificates(archive)
        require(installed.size==1 && incoming==installed && incoming.single()==release.certificate){"Signing certificate mismatch; keep installed app"}
        require(archive.applicationInfo?.minSdkVersion?.let{it<=Build.VERSION.SDK_INT}==true)
        java.util.zip.ZipFile(file).use{zip -> require(zip.getEntry("lib/arm64-v8a/libayshamusic.so")!=null){"Missing ARM64 backend"} }
    }
}
