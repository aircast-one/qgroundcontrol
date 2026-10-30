package one.aircast.android.ui

import one.aircast.android.installFailure
import one.aircast.android.installedFromPlay
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateTest {
    private fun reading(phase: String, canInstall: Boolean = true, snoozed: String = "", error: String = "", progress: Double = 0.0) =
        appUpdateReading(
            JSONObject(
                """{"class":"AppUpdate","current":"5.6.5-beta.21","channel":"beta","phase":"$phase","latest":"5.6.5-beta.22",
                "notes":"## Fixes\n\n- a [link](https://x) here\n\n**bold**","releaseUrl":"https://github.com/r","assetName":"AircastAndroid-v5.6.5-beta.22.apk",
                "assetUrl":"https://dl/a.apk","sizeBytes":151340969,"progress":$progress,"file":"/sdcard/Updates/a.apk","error":"$error",
                "checkedAt":"2026-10-31T10:00:00Z","autoCheck":true,"snoozed":"$snoozed","canInstall":$canInstall}""",
            ),
        )!!

    @Test
    fun `every phase has a status line and the latest version is named where it matters`() {
        assertEquals("", updateStatus(reading("idle"), fromPlay = false))
        assertEquals("Checking for updates…", updateStatus(reading("checking"), fromPlay = false))
        assertEquals("Aircast is up to date", updateStatus(reading("upToDate"), fromPlay = false))
        assertEquals("Aircast 5.6.5-beta.22 is available", updateStatus(reading("available"), fromPlay = false))
        assertEquals("Downloading Aircast 5.6.5-beta.22… 45%", updateStatus(reading("downloading", progress = 0.45), fromPlay = false))
        assertEquals("Aircast 5.6.5-beta.22 is ready to install", updateStatus(reading("ready"), fromPlay = false))
        assertEquals("a.apk did not match its published checksum", updateStatus(reading("failed", error = "a.apk did not match its published checksum"), fromPlay = false))
        assertEquals("The update check failed.", updateStatus(reading("failed"), fromPlay = false))
        assertEquals(UpdatePhase.Idle, reading("something new").phase)
    }

    @Test
    fun `an installer failure replaces the ready line and a snoozed version is remembered`() {
        assertEquals("INSTALL_FAILED_UPDATE_INCOMPATIBLE", updateStatus(reading("ready"), fromPlay = false, installError = "INSTALL_FAILED_UPDATE_INCOMPATIBLE"))
        assertTrue(reading("available", snoozed = "5.6.5-beta.22").snoozed)
        assertFalse(reading("available").snoozed)
    }

    @Test
    fun `the banner shows for an available, downloading or ready update and for nothing else`() {
        assertTrue(bannerShown(reading("available"), fromPlay = false))
        assertTrue(bannerShown(reading("downloading"), fromPlay = false))
        assertTrue(bannerShown(reading("ready"), fromPlay = false))
        listOf("idle", "checking", "upToDate", "failed").forEach { assertFalse(it, bannerShown(reading(it), fromPlay = false)) }
    }

    @Test
    fun `an open link hides the banner and blocks the install button`() {
        assertFalse(bannerShown(reading("available", canInstall = false), fromPlay = false))
        assertEquals(DISCONNECT_FIRST, installRefusal(reading("ready", canInstall = false)))
        assertNull(installRefusal(reading("ready")))
    }

    @Test
    fun `a snoozed update stays out of the banner but keeps its card`() {
        val snoozed = reading("available", snoozed = "5.6.5-beta.22")
        assertFalse(bannerShown(snoozed, fromPlay = false))
        assertEquals("Aircast 5.6.5-beta.22 is available", updateStatus(snoozed, fromPlay = false))
    }

    @Test
    fun `a Play install hides the banner and says so in the card`() {
        assertTrue(installedFromPlay("com.android.vending"))
        assertFalse(installedFromPlay("com.android.packageinstaller"))
        assertFalse(installedFromPlay(null))
        assertFalse(bannerShown(reading("available"), fromPlay = true))
        assertEquals(PLAY_UPDATES, updateStatus(reading("available"), fromPlay = true))
    }

    @Test
    fun `release notes lose their markdown but keep their lines`() {
        assertEquals("Fixes\n\na link here\n\nbold", plainNotes(reading("available").notes))
    }

    @Test
    fun `the installer's own message is shown and a cancelled install is named`() {
        assertEquals("INSTALL_FAILED_UPDATE_INCOMPATIBLE: signatures do not match", installFailure(5, "INSTALL_FAILED_UPDATE_INCOMPATIBLE: signatures do not match"))
        assertEquals("The install was cancelled.", installFailure(3, null))
        assertEquals("The installer refused the update (status 1).", installFailure(1, ""))
        assertEquals(1793440800000L, checkedAtMillis("2026-10-31T10:00:00Z"))
        assertNull(checkedAtMillis(""))
    }
}
