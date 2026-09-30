package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class EscIndicatorTest {
    @Test
    fun `the cell and page read the core's ESC summary`() {
        val summary = escSummary(
            JSONObject(
                """{"shown":true,"onlineCount":4,"healthy":true,"healthText":"OK","healthyMotorsText":"4/4","totalErrors":3,""" +
                    """"motors":[{"title":"Motor 1","healthy":true,"rpm":"4200 rpm","temperature":"","voltage":"16 V","current":"","errors":"1"}]}""",
            ),
        )!!
        assertEquals("ESC 4 OK", escCellText(summary))
        assertEquals("Motor 1", summary.motors[0].title)
        assertEquals("RPM" to "4200 rpm", summary.motors[0].rows[0])
        assertNull(escSummary(JSONObject("""{"shown":false}""")))
    }
}
