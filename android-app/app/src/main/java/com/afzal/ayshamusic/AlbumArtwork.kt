package com.afzal.ayshamusic

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import java.io.ByteArrayOutputStream

/** Bounded native JPEG crop; no network, model, storage permission or Activity. */
object AlbumArtwork {
    fun squareJpeg(bytes: ByteArray): ByteArray? {
        if (bytes.isEmpty() || bytes.size > 1024 * 1024) return null
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0 || bounds.outWidth > 8192 || bounds.outHeight > 8192 || bounds.outWidth.toLong() * bounds.outHeight > 16000000) return null
        var sample = 1
        while (maxOf(bounds.outWidth, bounds.outHeight) / sample > 640) sample *= 2
        val original = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = sample }) ?: return null
        var cropped: Bitmap? = null
        var square: Bitmap? = null
        return try {
            val side = minOf(original.width, original.height)
            cropped = Bitmap.createBitmap(original, (original.width - side) / 2, (original.height - side) / 2, side, side)
            square = Bitmap.createScaledBitmap(cropped, 320, 320, true)
            val out = ByteArrayOutputStream()
            if (!square.compress(Bitmap.CompressFormat.JPEG, 85, out)) null
            else out.toByteArray().takeIf { it.size <= 200000 }
        } finally {
            if (square !== cropped && square !== original) square?.recycle()
            if (cropped !== original) cropped?.recycle()
            original.recycle()
        }
    }
}
