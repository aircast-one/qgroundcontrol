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
            "timeText":"2026-09-08 01:20:00"},
           {"index":1,"id":2,"sizeBytes":10240,"sizeText":"10.0KB","status":"Available",
            "received":true,"selected":true,"time":"2026-09-08T02:20:00",
            "timeText":"2026-09-08 02:20:00"}]}
    """

    @Test
    fun `the entries carry the core's formatted size and time`() {
        val logs = logsView(JSONObject(served))!!

        assertEquals(listOf("4.0KB", "10.0KB"), logs.entries.map { it.sizeStr })
        assertEquals(listOf("2026-09-08 01:20:00", "2026-09-08 02:20:00"), logs.entries.map { it.time })
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
