package one.aircast.android.ui

import android.content.pm.PackageInstaller
import one.aircast.android.InstallFailure
import one.aircast.android.InstallOutcome
import one.aircast.android.installFailure
import one.aircast.android.installedFromPlay
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUpdateTest {
    private val apk = "/sdcard/Updates/AircastAndroid-v5.6.5-beta.22.apk"

    private fun reading(
        phase: String,
        canInstall: Boolean = true,
        skipped: String = "",
        later: Boolean = false,
        updatedFrom: String = "",
        error: String = "",
        progress: Double = 0.0,
        latest: String = "5.6.5-beta.22",
        file: String = apk,
    ) = appUpdateReading(
        JSONObject(
            """{"class":"AppUpdate","current":"5.6.5-beta.21","channel":"beta","phase":"$phase","latest":"$latest",
            "notes":"## Fixes\n\n- a [link](https://x) here\n\n**bold**","releaseUrl":"https://github.com/r","assetName":"AircastAndroid-v5.6.5-beta.22.apk",
            "assetUrl":"https://dl/a.apk","sizeBytes":151340969,"progress":$progress,"file":"$file","error":"$error",
            "checkedAt":"2026-10-31T10:00:00Z","autoCheck":true,"skipped":"$skipped","later":$later,"updatedFrom":"$updatedFrom","canInstall":$canInstall}""",
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
    fun `the banner shows for an available, downloading or ready update and for nothing else`() {
        assertTrue(bannerShown(reading("available"), fromPlay = false))
        assertTrue(bannerShown(reading("downloading"), fromPlay = false))
        assertTrue(bannerShown(reading("ready"), fromPlay = false))
        listOf("idle", "checking", "upToDate", "failed").map { assertFalse(it, bannerShown(reading(it), fromPlay = false)) }
    }

    @Test
    fun `an open link hides the banner and blocks the install button`() {
        assertFalse(bannerShown(reading("available", canInstall = false), fromPlay = false))
        assertEquals(DISCONNECT_FIRST, installRefusal(reading("ready", canInstall = false)))
        assertNull(installRefusal(reading("ready")))
    }

    @Test
    fun `later hides the banner for this run and a skip hides only the version it named`() {
        assertFalse(bannerShown(reading("available", later = true), fromPlay = false))
        assertFalse(bannerShown(reading("available", skipped = "5.6.5-beta.22"), fromPlay = false))
        assertTrue(skippedThis(reading("ready", skipped = "5.6.5-beta.22")))
        assertTrue(bannerShown(reading("available", skipped = "5.6.5-beta.21"), fromPlay = false))
        assertEquals("Aircast 5.6.5-beta.22 is available", updateStatus(reading("available", skipped = "5.6.5-beta.22"), fromPlay = false))
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
    fun `the upgrade is reported once, not while a vehicle is connected, and links to the release that is running`() {
        assertTrue(updatedShown(reading("upToDate", updatedFrom = "5.6.5-beta.20")))
        assertFalse(updatedShown(reading("upToDate")))
        assertFalse(updatedShown(reading("upToDate", updatedFrom = "5.6.5-beta.20", canInstall = false)))
        assertEquals("https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v5.6.5-beta.21", releaseTagUrl(reading("upToDate").current))
    }

    @Test
    fun `installer failures become plain sentences and never the raw code`() {
        val mismatch = "INSTALL_FAILED_UPDATE_INCOMPATIBLE: Existing package one.aircast.app signatures do not match newer version; ignoring!"
        assertEquals(InstallFailure.DifferentBuild, installFailure(PackageInstaller.STATUS_FAILURE_CONFLICT, mismatch))
        assertEquals(InstallFailure.DifferentBuild, installFailure(PackageInstaller.STATUS_FAILURE_INCOMPATIBLE, null))
        assertEquals(InstallFailure.DifferentBuild, installFailure(PackageInstaller.STATUS_FAILURE, "signatures do not match"))
        val downgrade = "INSTALL_FAILED_VERSION_DOWNGRADE: Downgrade detected: Update version code 153 is older than current 5060005"
        assertEquals(InstallFailure.Downgrade, installFailure(PackageInstaller.STATUS_FAILURE_INCOMPATIBLE, downgrade))
        assertEquals(InstallFailure.NoSpace, installFailure(PackageInstaller.STATUS_FAILURE_STORAGE, "INSTALL_FAILED_INSUFFICIENT_STORAGE"))
        assertEquals(InstallFailure.Refused, installFailure(PackageInstaller.STATUS_FAILURE_INVALID, "INSTALL_PARSE_FAILED_NOT_APK"))
        assertNull(installFailure(PackageInstaller.STATUS_SUCCESS, null))
        assertNull(installFailure(PackageInstaller.STATUS_FAILURE_ABORTED, null))
        assertTrue(InstallFailure.entries.none { it.text.contains("INSTALL_") })
        assertEquals(InstallFailure.DifferentBuild.text, updateStatus(reading("ready"), fromPlay = false, failure = InstallFailure.DifferentBuild))
    }

    @Test
    fun `each failure offers its own way forward`() {
        assertEquals(ReadyAction.OpenReleasePage, readyAction(InstallFailure.DifferentBuild))
        assertEquals(ReadyAction.None, readyAction(InstallFailure.Downgrade))
        assertEquals(ReadyAction.Install, readyAction(InstallFailure.NoSpace))
        assertEquals(ReadyAction.Install, readyAction(InstallFailure.Refused))
        assertEquals(ReadyAction.Install, readyAction(null))
        assertEquals("Try again", installLabel(InstallFailure.Refused))
        assertEquals("Install", installLabel(null))
    }

    @Test
    fun `a stored install failure only stands beside the ready file it came from`() {
        val failed = InstallOutcome(apk, InstallFailure.DifferentBuild)
        assertEquals(InstallFailure.DifferentBuild, installFailureFor(failed, reading("ready")))
        assertNull(installFailureFor(failed, reading("ready", canInstall = false)))
        assertNull(installFailureFor(failed, reading("available")))
        assertNull(installFailureFor(failed, reading("ready", latest = "5.6.5-beta.23", file = "/sdcard/Updates/AircastAndroid-v5.6.5-beta.23.apk")))
        assertNull(installFailureFor(null, reading("ready")))
    }

    @Test
    fun `the size rides on the buttons and in the metered question`() {
        assertEquals("Update (151 MB)", sizedLabel("Update", "151 MB"))
        assertEquals("Download", sizedLabel("Download", null))
        assertEquals("This update is 151 MB and you're on a metered connection. Download now?", meteredQuestion("151 MB"))
        assertEquals("You're on a metered connection. Download the update now?", meteredQuestion(null))
    }

    @Test
    fun `release notes lose their markdown but keep their lines`() {
        assertEquals("Fixes\n\na link here\n\nbold", plainNotes(reading("available").notes))
        assertEquals(1793440800000L, checkedAtMillis("2026-10-31T10:00:00Z"))
        assertNull(checkedAtMillis(""))
    }
}
