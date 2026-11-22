package com.afzal.ayshamusic

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

/** Offline device checks only: no real release, signing continuity or phone launcher implied. */
@RunWith(AndroidJUnit4::class)
class IconUpdateTest {
    @Test fun launcherAliasesCanSwitchAndRestore() {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val original=AppIcons.current(context)
        try {
            AppIcons.choices.forEach { choice ->
                AppIcons.select(context,choice.id)
                assertEquals(choice.id,AppIcons.current(context))
            }
            try { AppIcons.select(context,"not-an-icon");fail("unknown icon accepted") } catch (_:IllegalArgumentException) { }
        } finally { AppIcons.select(context,original) }
    }
    @Test fun updateMetadataStrictlyRequiresOfficialAssetAndBoundedValues() {
        val json=JSONObject().put("schema",1).put("package_id","com.afzal.ayshamusic.androidaudio10")
            .put("abi","arm64-v8a").put("min_sdk",26).put("version_code",1001)
            .put("version_name","1.9.0.1001").put("size",1000)
            .put("sha256","a".repeat(64)).put("certificate_sha256","b".repeat(64))
            .put("url","https://github.com/AfzalAshraf/ayshamusic/releases/download/android-v1001/AyshaMusic-arm64.apk")
        assertEquals(1001L,UpdatePolicy.parse(json).code)
        json.put("url","https://example.org/a.apk")
        try { UpdatePolicy.parse(json);fail("foreign APK URL accepted") } catch (_:IllegalArgumentException) { }
        assertFalse(UpdatePolicy.allowedConnection("http://github.com/anything"))
        assertFalse(UpdatePolicy.allowedConnection("https://github.com.evil.example/file"))
        assertTrue(UpdatePolicy.allowedConnection("https://release-assets.githubusercontent.com/file"))
        assertEquals(1,UpdatePolicy.certificates(UpdatePolicy.info(InstrumentationRegistry.getInstrumentation().targetContext)).size)
    }
}
