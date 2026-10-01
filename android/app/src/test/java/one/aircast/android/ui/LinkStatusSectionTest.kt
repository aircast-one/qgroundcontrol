package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class LinkStatusSectionTest {
    @Test
    fun `link status rows read in order`() {
        val view = JSONObject("""{"rows":[{"label":"Messages Sent","value":"62"},{"label":"Loss Rate","value":"3%"}]}""")
        assertEquals(listOf("Messages Sent" to "62", "Loss Rate" to "3%"), linkStatusRows(view))
        assertEquals(emptyList<Pair<String, String>>(), linkStatusRows(null))
    }
}
