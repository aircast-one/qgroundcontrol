package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class VideoStatsPillTest {
    @Test
    fun `the pill shows the core's stats line`() {
        assertEquals("30 fps · 720p", videoStatsText(JSONObject("""{"text":"30 fps · 720p"}""")))
        assertEquals("", videoStatsText(null))
    }
}
