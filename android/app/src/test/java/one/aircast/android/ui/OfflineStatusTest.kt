package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OfflineStatusTest {
    @Test
    fun `the offline page reads the core's footnote and links`() {
        val status = offlineStatus(
            JSONObject(
                """{"title":"Connection Failed","footnote":"Couldn't connect to Drone.","busy":false,"noLinks":false,"editAddress":true,
                   "links":[{"index":2,"text":"Connect to Drone","description":"TCP\nrefused","retry":true,"failed":true,"connected":false}]}""",
            ),
        )
        assertEquals("Connection Failed", status?.title)
        assertEquals(true, status?.editAddress)
        assertEquals(listOf(OfflineLink(2, "Connect to Drone", "TCP\nrefused", retry = true, failed = true, connected = false)), status?.links)
        assertNull(offlineStatus(JSONObject("{}")))
    }
}
