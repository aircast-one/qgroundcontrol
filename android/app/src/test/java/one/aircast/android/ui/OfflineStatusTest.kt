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
                   "signInUrl":"https://aircast.one/device?code=ABCD",
                   "links":[{"index":2,"text":"Drone","description":"TCP\nrefused","action":"edit","actionLabel":"Edit","failed":true,"connected":false}]}""",
            ),
        )
        assertEquals("Connection Failed", status?.title)
        assertEquals(true, status?.editAddress)
        assertEquals("https://aircast.one/device?code=ABCD", status?.signInUrl)
        assertEquals(listOf(OfflineLink(2, "Drone", "TCP\nrefused", action = "edit", actionLabel = "Edit", failed = true, connected = false)), status?.links)
        assertNull(offlineStatus(JSONObject("{}")))
    }
}
