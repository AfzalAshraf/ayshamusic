package com.afzal.ayshamusic

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.MediaStore
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/** Only files published to Music/Aysha Music with our exact ID-based names. */
object LocalMusic {
    private val filename=Regex("^Aysha-([A-Za-z0-9_-]{11})\\.m4a$")
    fun hasReadAccess(context:Context):Boolean {
        val permission=if(Build.VERSION.SDK_INT>=33) Manifest.permission.READ_MEDIA_AUDIO else Manifest.permission.READ_EXTERNAL_STORAGE
        return ContextCompat.checkSelfPermission(context,permission)==PackageManager.PERMISSION_GRANTED
    }
    private fun safeTitle(value:String?,fallback:String,max:Int):String =
        (value ?: "").filterNot{it.isISOControl()}.take(max).trim().let{if(it.isEmpty() || it=="<unknown>")fallback else it}
    fun list(context:Context):String {
        val items=JSONArray();val seen=HashSet<String>()
        try {
            if(Build.VERSION.SDK_INT>=29) {
                val collection=MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
                val columns=arrayOf(MediaStore.Audio.Media.DISPLAY_NAME,MediaStore.Audio.Media.TITLE,MediaStore.Audio.Media.ARTIST)
                val where="${MediaStore.Audio.Media.RELATIVE_PATH}=? AND ${MediaStore.Audio.Media.IS_PENDING}=0"
                context.contentResolver.query(collection,columns,where,arrayOf(Environment.DIRECTORY_MUSIC+"/Aysha Music/"),null)?.use{cursor->
                    while(items.length()<500 && cursor.moveToNext()) {
                        val id=filename.matchEntire(cursor.getString(0) ?: "")?.groupValues?.get(1) ?: continue
                        if(!seen.add(id))continue
                        val title=safeTitle(cursor.getString(1),"Aysha song",300);val artist=safeTitle(cursor.getString(2),"",150)
                        items.put(JSONObject().put("id",id).put("title",title).put("artist",artist))
                    }
                }
            } else {
                @Suppress("DEPRECATION") val dir=File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_MUSIC),"Aysha Music")
                for(file in dir.listFiles().orEmpty().sortedBy{it.name}.take(1000)) {
                    val id=filename.matchEntire(file.name)?.groupValues?.get(1) ?: continue
                    if(!file.isFile || !seen.add(id))continue
                    items.put(JSONObject().put("id",id).put("title","Aysha song · $id").put("artist",""))
                    if(items.length()>=500)break
                }
            }
            return JSONObject().put("items",items).put("permission_required",!hasReadAccess(context)).toString()
        }catch(_:Exception){return JSONObject().put("items",items).put("permission_required",!hasReadAccess(context)).put("error","Could not read the Music folder").toString()}
    }
    /** Return a detached read-only descriptor; Rust owns and closes it. */
    fun open(context:Context,id:String):Int {
        if(!Regex("^[A-Za-z0-9_-]{11}$").matches(id))return -1
        return try {
            val name="Aysha-$id.m4a"
            if(Build.VERSION.SDK_INT>=29) {
                val collection=MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
                val where="${MediaStore.Audio.Media.DISPLAY_NAME}=? AND ${MediaStore.Audio.Media.RELATIVE_PATH}=? AND ${MediaStore.Audio.Media.IS_PENDING}=0"
                context.contentResolver.query(collection,arrayOf(MediaStore.Audio.Media._ID),where,arrayOf(name,Environment.DIRECTORY_MUSIC+"/Aysha Music/"),null)?.use{cursor->
                    if(cursor.moveToFirst()) {
                        val uri=ContentUris.withAppendedId(collection,cursor.getLong(0))
                        context.contentResolver.openFileDescriptor(uri,"r")?.use{return it.detachFd()}
                    }
                }
                -1
            } else {
                @Suppress("DEPRECATION") val dir=File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_MUSIC),"Aysha Music").canonicalFile
                val file=File(dir,name).canonicalFile
                if(file.parentFile!=dir || !file.isFile)return -1
                ParcelFileDescriptor.open(file,ParcelFileDescriptor.MODE_READ_ONLY).use{it.detachFd()}
            }
        }catch(_:Exception){-1}
    }
}
