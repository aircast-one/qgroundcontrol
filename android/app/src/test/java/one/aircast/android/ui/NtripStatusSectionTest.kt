package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class NtripStatusSectionTest {
    @Test
    fun counts_and_sizes_read_like_qgc() {
        assertNull(ntripStatus(JSONObject("{}")))
        val status = ntripStatus(JSONObject("""{"status":"connected","statusMessage":"Connected","messageTypes":[[1005,3],[0,1]],"bytesReceived":2048}"""))!!
        assertEquals(listOf(1005 to 3L, 0 to 1L), status.messageTypes)
        assertEquals("2.0 KB", dataSize(status.bytesReceived))
        assertEquals("512 B", dataSize(512))
        assertEquals("1.5 MB", dataSize(1572864))
        assertEquals("900 B/s", dataRate(900.0))
        assertEquals("2.0 KB/s", dataRate(2048.0))
    }

    @Test
    fun the_browser_lists_mountpoints_and_marks_the_selected_one() {
        val browser = ntripBrowser(JSONObject("""{"status":"success","error":"","canBrowse":true,"mountpoints":[{"mountpoint":"NEAR","detail":"RTCM 3.3 · GPS","selected":true}]}"""))
        assertEquals(NtripMountpointRow("NEAR", "RTCM 3.3 · GPS", true), browser.mountpoints.single())
        assertEquals("", ntripBrowser(null).status)
    }
}
