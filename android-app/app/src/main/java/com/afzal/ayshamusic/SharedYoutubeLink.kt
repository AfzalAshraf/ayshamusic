package com.afzal.ayshamusic

import android.net.Uri

/** Extract an ID, never fetch or execute a caller-supplied URL. */
object SharedYoutubeLink {
    private val idPattern = Regex("^[A-Za-z0-9_-]{11}$")
    fun videoId(text: String?): String? {
        if (text == null || text.length > 4096) return null
        val links = Regex("https://[^\\s<>]+", RegexOption.IGNORE_CASE).findAll(text).take(2).toList()
        if (links.size != 1) return null
        val uri = Uri.parse(links[0].value.trimEnd('.', ',', ')', ']', '!', '?'))
        if (!uri.scheme.equals("https", true) || uri.userInfo != null || uri.port != -1 || uri.fragment != null) return null
        val host = uri.host?.lowercase() ?: return null
        val segments = uri.pathSegments
        val id = when (host) {
            "youtu.be" -> if (segments.size == 1) segments[0] else null
            "youtube.com", "www.youtube.com", "m.youtube.com", "music.youtube.com" -> when {
                uri.path == "/watch" && uri.getQueryParameters("v").size == 1 -> uri.getQueryParameter("v")
                segments.size == 2 && segments[0] in listOf("shorts", "live", "embed") -> segments[1]
                else -> null
            }
            else -> null
        }
        return id?.takeIf { idPattern.matches(it) }
    }
}
