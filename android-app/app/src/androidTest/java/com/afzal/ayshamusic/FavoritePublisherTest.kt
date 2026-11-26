package com.afzal.ayshamusic

import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/** Offline MediaStore fixture; it does not prove real provider/phone downloading. */
@RunWith(AndroidJUnit4::class)
class FavoritePublisherTest {
    @Test fun scopedPublicationWritesOnePublicFileAndDoesNotOverwrite() {
        assumeTrue(Build.VERSION.SDK_INT>=29)
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val id="TEST0000001";val name="Aysha-$id.m4a"
        val collection=MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val where="${MediaStore.Audio.Media.DISPLAY_NAME}=? AND ${MediaStore.Audio.Media.RELATIVE_PATH}=?"
        val args=arrayOf(name,Environment.DIRECTORY_MUSIC+"/Aysha Music/")
        fun existing():Long?=context.contentResolver.query(collection,arrayOf(MediaStore.Audio.Media._ID),where,args,null)?.use{if(it.moveToFirst())it.getLong(0) else null}
        assumeTrue("fixture name already exists; do not alter user content",existing()==null)
        val dir=File(context.cacheDir,"favorite-downloads").apply{mkdirs()};val file=File(dir,"$id.part")
        val bytes=byteArrayOf(0,0,0,24,102,116,121,112,77,52,65,32);file.writeBytes(bytes)
        var created:android.net.Uri?=null
        try {
            assertTrue(FavoritePublisher.publish(context,file.absolutePath,id,"Fixture song","Fixture artist"))
            val row=existing();assertNotNull(row);created=android.content.ContentUris.withAppendedId(collection,row!!)
            assertArrayEquals(bytes,context.contentResolver.openInputStream(created!!).use{it!!.readBytes()})
            file.writeBytes(byteArrayOf(1,2,3));assertTrue(FavoritePublisher.publish(context,file.absolutePath,id,"Fixture song","Fixture artist"))
            assertArrayEquals(bytes,context.contentResolver.openInputStream(created!!).use{it!!.readBytes()})
            assertFalse(FavoritePublisher.publish(context,file.absolutePath,"../../bad","x",""))
        }finally{created?.let{context.contentResolver.delete(it,null,null)};file.delete()}
    }
}
