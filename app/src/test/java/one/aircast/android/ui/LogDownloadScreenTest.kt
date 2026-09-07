package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class LogDownloadScreenTest {
    @Test
    fun `entries are parsed from the model elements`() {
        val model = JSONObject(
            """
            {"count":2,"elements":[
              {"id":3,"time":"2026-09-06T23:48:14","sizeStr":"2.4MB","received":true,"selected":false,"status":"Available"},
              {"id":4,"time":"2026-09-06T23:59:00","sizeStr":"1.0MB","received":false,"selected":true,"status":"Pending"}
            ]}
            """.trimIndent(),
        )
        val entries = parseLogEntries(model)
        assertEquals(2, entries.size)
        assertEquals(LogEntry(0, 3, "2026-09-06T23:48:14", "2.4MB", true, false, "Available"), entries[0])
        assertTrue(entries[1].selected)
        assertEquals(1, entries[1].index)
    }

    @Test
    fun `a model without elements yields nothing`() {
        assertEquals(emptyList<LogEntry>(), parseLogEntries(null))
        assertEquals(emptyList<LogEntry>(), parseLogEntries(JSONObject("""{"kind":"object"}""")))
    }

    @Test
    fun `missing fields fall back instead of throwing`() {
        val entries = parseLogEntries(JSONObject("""{"elements":[{}]}"""))
        assertEquals(1, entries.size)
        assertEquals(0, entries[0].id)
        assertEquals("", entries[0].status)
    }

    @Test
    fun `log time drops the iso separator and fractional seconds`() {
        assertEquals("2026-09-06 23:48:14", formatLogTime("2026-09-06T23:48:14.123"))
        assertEquals("2026-09-06 23:48:14", formatLogTime("2026-09-06T23:48:14"))
    }

    @Test
    fun `an empty time reads as unknown`() {
        assertEquals("Unknown date", formatLogTime(""))
    }

    @Test
    fun `logs are fetched on arrival only when there is nothing to show and nothing running`() {
        assertEquals(true, shouldAutoRefreshLogs(hasVehicle = true, hasEntries = false, busy = false))
        assertEquals(false, shouldAutoRefreshLogs(hasVehicle = false, hasEntries = false, busy = false))
    }

    @Test
    fun `arriving back mid-download does not restart the listing`() {
        assertEquals(false, shouldAutoRefreshLogs(hasVehicle = true, hasEntries = false, busy = true))
        assertEquals(false, shouldAutoRefreshLogs(hasVehicle = true, hasEntries = true, busy = false))
    }
}
