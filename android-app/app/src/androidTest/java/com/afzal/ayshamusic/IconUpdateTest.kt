package com.afzal.ayshamusic

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
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
}
