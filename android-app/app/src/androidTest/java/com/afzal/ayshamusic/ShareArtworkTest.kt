package com.afzal.ayshamusic

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream

@RunWith(AndroidJUnit4::class)
class ShareArtworkTest {
    @Test fun acceptsOnlySingleCanonicalYoutubeVideoLinks() {
        // The offline x86 emulator deliberately excludes the Rust JNI library.
        // Run these bridge checks on an ARM64 device/build that includes it.
        org.junit.Assume.assumeTrue(android.os.Build.SUPPORTED_ABIS.contains("arm64-v8a"))
        for (url in listOf("https://youtu.be/abcdefghijk?si=abc", "Watch https://www.youtube.com/watch?v=abcdefghijk", "https://music.youtube.com/watch?v=abcdefghijk&list=test", "https://youtube.com/shorts/abcdefghijk", "https://m.youtube.com/live/abcdefghijk")) assertEquals(url,"abcdefghijk",RustBridge.sharedVideoId(url))
        for (url in listOf("https://youtube.com.evil.test/watch?v=abcdefghijk", "https://youtube.com@evil.test/watch?v=abcdefghijk", "http://youtu.be/abcdefghijk", "https://youtube.com/watch?v=abcdefghijk&v=another0001", "https://youtube.com/playlist?list=x", "https://youtu.be/short", "https://youtu.be/abcdefghijk https://youtu.be/nexttrack01", "https://youtube.com:444/watch?v=abcdefghijk", "javascript:alert(1)", "x".repeat(5000))) assertNull(url,RustBridge.sharedVideoId(url))
    }
    @Test fun squareArtworkIsBoundedJpegAndRejectsInvalidInput() {
        assertNull(AlbumArtwork.squareJpeg(byteArrayOf(1,2,3)))
        assertNull(AlbumArtwork.squareJpeg(ByteArray(1024*1024+1)))
        val original=Bitmap.createBitmap(480,270,Bitmap.Config.ARGB_8888)
        original.eraseColor(android.graphics.Color.RED)
        val stream=ByteArrayOutputStream();original.compress(Bitmap.CompressFormat.JPEG,90,stream);original.recycle()
        val square=AlbumArtwork.squareJpeg(stream.toByteArray());assertNotNull(square);assertTrue(square!!.size<=200000)
        val decoded=BitmapFactory.decodeByteArray(square,0,square.size)
        assertEquals(320,decoded.width);assertEquals(320,decoded.height);decoded.recycle()
    }
}
