package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class Px4AirframeScreenTest {
    @Test
    fun `the current airframe preselects its group and the apply text loses its html breaks`() {
        val read = px4Airframes(
            JSONObject(
                """{"available":true,"autostartId":4001,"custom":false,"heading":"You've connected a Generic Quadcopter.","currentType":"Quadrotor x",""" +
                    """"currentIndex":0,"applyTitle":"Apply and Restart","applyText":"a<br><br>b","types":[{"name":"Quadrotor x","airframes":[{"name":"Generic Quadcopter","autostartId":4001}]}]}""",
            ),
        )!!
        assertEquals(AirframeSelection("Quadrotor x", 0), initialSelection(read))
        assertEquals("a\n\nb", read.applyText)
        assertEquals(4001L, read.groups.single().airframes.single().autostartId)
        assertNull(px4Airframes(JSONObject("""{"available":false}""")))
    }
}
