package com.afzal.ayshamusic

import android.Manifest
import android.content.ContentValues
import android.content.Context
import android.content.pm.PackageManager
import android.media.MediaScannerConnection
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import java.io.File

/** Copies a completed Rust staging file to public Music; never deletes an existing song. */
object FavoritePublisher {
    fun canPublish(context:Context):Boolean = Build.VERSION.SDK_INT>=29 ||
        androidx.core.content.ContextCompat.checkSelfPermission(context,Manifest.permission.WRITE_EXTERNAL_STORAGE)==PackageManager.PERMISSION_GRANTED
    fun publish(context:Context,path:String,id:String,title:String,artist:String):Boolean {
        if(!Regex("^[A-Za-z0-9_-]{11}$").matches(id))return false
        val root=File(context.cacheDir,"favorite-downloads").canonicalFile
        val source=File(path).canonicalFile
        if(source.parentFile!=root || source.name!="$id.part" || !source.isFile || source.length() !in 1..(64L*1024*1024))return false
        val name="Aysha-$id.m4a"
        val cleanTitle=title.take(300).filterNot{it.isISOControl()}.ifBlank{"Aysha song"}
        val cleanArtist=artist.take(150).filterNot{it.isISOControl()}
        return try {
            if(Build.VERSION.SDK_INT>=29) publishScoped(context,source,name,cleanTitle,cleanArtist)
            else publishLegacy(context,source,name)
        }catch(_:Exception){false}
    }
    private fun publishScoped(context:Context,source:File,name:String,title:String,artist:String):Boolean {
        val resolver=context.contentResolver
        val collection=MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val relative=Environment.DIRECTORY_MUSIC+"/Aysha Music/"
        resolver.query(collection,arrayOf(MediaStore.Audio.Media._ID),"${MediaStore.Audio.Media.DISPLAY_NAME}=? AND ${MediaStore.Audio.Media.RELATIVE_PATH}=? AND ${MediaStore.Audio.Media.IS_PENDING}=0",arrayOf(name,relative),null)?.use{if(it.moveToFirst())return true}
        val values=ContentValues().apply {
            put(MediaStore.Audio.Media.DISPLAY_NAME,name);put(MediaStore.Audio.Media.MIME_TYPE,"audio/mp4")
            put(MediaStore.Audio.Media.TITLE,title);put(MediaStore.Audio.Media.ARTIST,artist)
            put(MediaStore.Audio.Media.RELATIVE_PATH,relative);put(MediaStore.Audio.Media.IS_PENDING,1)
        }
        val uri=resolver.insert(collection,values) ?: return false
        var complete=false
        try {
            val out=resolver.openOutputStream(uri,"w") ?: return false
            out.use{output->source.inputStream().use{input->input.copyTo(output,64*1024)}}
            val publish=ContentValues().apply{put(MediaStore.Audio.Media.IS_PENDING,0)}
            complete=resolver.update(uri,publish,null,null)==1
            return complete
        }finally{if(!complete)try{resolver.delete(uri,null,null)}catch(_:Exception){}}
    }
    @Suppress("DEPRECATION") private fun publishLegacy(context:Context,source:File,name:String):Boolean {
        if(androidx.core.content.ContextCompat.checkSelfPermission(context,Manifest.permission.WRITE_EXTERNAL_STORAGE)!=PackageManager.PERMISSION_GRANTED)return false
        val dir=File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_MUSIC),"Aysha Music")
        if(!dir.isDirectory && !dir.mkdirs())return false
        val dest=File(dir,name);if(dest.exists())return true
        val part=File(dir,".$name.pending")
        try {
            source.inputStream().use{input->part.outputStream().use{output->input.copyTo(output,64*1024);output.fd.sync()}}
            if(dest.exists()){part.delete();return true}
            if(!part.renameTo(dest))return false
            MediaScannerConnection.scanFile(context,arrayOf(dest.absolutePath),arrayOf("audio/mp4"),null)
            return true
        }finally{if(part.exists())part.delete()}
    }
}
