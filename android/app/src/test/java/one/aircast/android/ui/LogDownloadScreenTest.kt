package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LogDownloadScreenTest {

    private val served = """
        {"connected":true,"busy":false,"canRefresh":true,"canDownload":true,
         "canCancel":false,"canErase":true,"anyDownloaded":true,
         "emptyText":"","eraseWarning":"This erases every log on the vehicle.",
         "entries":[
           {"index":0,"id":1,"sizeBytes":4096,"sizeText":"4.0KB","status":"Downloaded",
            "received":true,"selected":false,"time":"2026-09-08T01:20:00",
            "timeState":"known"},
           {"index":1,"id":2,"sizeBytes":10240,"sizeText":"10.0KB","status":"Available",
            "received":true,"selected":true,"time":"2026-09-08T02:20:00",
            "timeState":"known"}]}
    """

    @Test
    fun `the entries carry the core's formatted size and time`() {
        val logs = logsView(JSONObject(served))!!

        assertEquals(listOf("4.0KB", "10.0KB"), logs.entries.map { it.sizeStr })
        assertTrue(logs.entries.all { it.time.isNotBlank() && it.time != "Date unknown" })
        assertEquals(listOf(1, 2), logs.entries.map { it.id })
        assertEquals(listOf("Downloaded", "Available"), logs.entries.map { it.status })
    }

    @Test
    fun `the buttons follow the core rather than the head's own arithmetic`() {
        val logs = logsView(JSONObject(served))!!

        assertTrue(logs.canRefresh)
        assertTrue(logs.canDownload)
        assertFalse(logs.canCancel)
        assertTrue(logs.anyDownloaded)
        assertEquals("This erases every log on the vehicle.", logs.eraseWarning)
    }

    @Test
    fun `a vehicle with no logs is connected with an empty list`() {
        val logs = logsView(
            JSONObject("""{"connected":true,"entries":[],"emptyText":"This vehicle reports no flight logs."}"""),
        )!!

        assertTrue(logs.connected)
        assertEquals(emptyList<LogEntry>(), logs.entries)
        assertEquals("This vehicle reports no flight logs.", logs.emptyText)
    }

    @Test
    fun `no view at all is no screen state`() {
        assertNull(logsView(null))
    }

    @Test
    fun `the auto refresh only fires for a connected vehicle with nothing listed`() {
        assertTrue(shouldAutoRefreshLogs(hasVehicle = true, hasEntries = false, busy = false))
        assertFalse(shouldAutoRefreshLogs(hasVehicle = true, hasEntries = true, busy = false))
        assertFalse(shouldAutoRefreshLogs(hasVehicle = true, hasEntries = false, busy = true))
        assertFalse(shouldAutoRefreshLogs(hasVehicle = false, hasEntries = false, busy = false))
    }
}

class LogTimeTextTest {
    private val fixed: (java.time.LocalDateTime) -> String = { "FORMATTED" }

    @Test
    fun `an unreceived entry shows no time`() {
        assertEquals("", logTimeText("2026-09-10T12:00:00Z", TIME_UNRECEIVED, fixed))
    }

    @Test
    fun `a vehicle with an unset clock says so rather than showing 1970`() {
        assertEquals("Date unknown", logTimeText("1970-01-01T00:00:00Z", TIME_UNKNOWN, fixed))
    }

    @Test
    fun `a known time is handed to the local formatter`() {
        assertEquals("FORMATTED", logTimeText("2026-09-10T12:00:00Z", "known", fixed))
    }

    @Test
    fun `an unparseable time does not crash the list`() {
        assertEquals("Date unknown", logTimeText("not a time", "known", fixed))
    }

    @Test
    fun `a Qt local time with no zone designator is what the bridge actually sends`() {
        val seen = mutableListOf<java.time.LocalDateTime>()
        logTimeText("2026-09-08T01:20:00", "known") { seen.add(it); "x" }
        assertEquals(java.time.LocalDateTime.parse("2026-09-08T01:20:00"), seen.single())
    }

    @Test
    fun `an offset form is converted rather than rejected`() {
        assertEquals("FORMATTED", logTimeText("2026-09-08T01:20:00+03:00", "known", fixed))
    }
}

class LogTimeZoneTest {
    @Test
    fun `a zone-less time is wall clock and must not be read as UTC`() {
        assertEquals(
            java.time.LocalDateTime.of(2026, 9, 8, 14, 42, 51),
            logLocalTime("2026-09-08T14:42:51"),
        )
    }

    @Test
    fun `an offset time is converted into the reader's zone`() {
        val expected = java.time.OffsetDateTime.parse("2026-09-08T14:42:51+04:00")
            .atZoneSameInstant(java.time.ZoneId.systemDefault())
            .toLocalDateTime()
        assertEquals(expected, logLocalTime("2026-09-08T14:42:51+04:00"))
    }
}

class LogSavePathTest {

    private fun logs(anyDownloaded: Boolean, savePath: String, reason: String) = logsView(
        JSONObject(
            """{"kind":"object","class":"LogDownload","entries":[],"busy":false,
               "anyDownloaded":$anyDownloaded,"savePath":$savePath,"savePathReason":$reason}""",
        ),
    )

    @Test
    fun `nothing downloaded says nothing about where it went`() {
        assertNull(savedToText(logs(false, "\"/sdcard/Logs\"", "null")))
        assertNull(savedToText(null))
    }

    @Test
    fun `a known directory is named`() {
        assertEquals("Saved to /sdcard/Logs", savedToText(logs(true, "\"/sdcard/Logs\"", "null")))
    }

    @Test
    fun `an empty path with a reason says the reason, because the two empties differ`() {
        assertEquals(
            "The folder chosen for logs is no longer there.",
            savedToText(logs(true, "\"\"", "\"The folder chosen for logs is no longer there.\"")),
        )
    }

    @Test
    fun `an empty path with no reason stays silent rather than printing Saved to nowhere`() {
        assertNull(savedToText(logs(true, "\"\"", "null")))
    }
}
