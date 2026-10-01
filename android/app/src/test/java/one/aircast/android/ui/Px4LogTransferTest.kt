package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class Px4LogTransferTest {
    @Test
    fun `the page reads the logging state, settings and saved files`() {
        val log = mavlinkLog(JSONObject("""{"vehiclePx4":true,"logRunning":false,"canStartLog":true,"persistence":true,"settings":{"emailAddress":"a@b.c","windSpeed":"5"},"files":[{"name":"001-x.ulg","size":2048,"uploaded":true}],"uploading":false,"uploadingFile":null,"message":null}"""))!!
        assertTrue(log.canStart)
        assertEquals(listOf(LogFile("001-x.ulg", 2048, true)), log.files)
        assertEquals("a@b.c", log.settings.optString("emailAddress"))
        assertEquals("Breeze", WIND_SPEEDS.first { it.second == log.settings.optString("windSpeed") }.first)
    }
}
