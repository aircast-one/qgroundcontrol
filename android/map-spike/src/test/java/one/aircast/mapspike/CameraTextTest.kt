package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CameraTextTest {

    private fun stats(json: String) = surveyStats(JSONObject(json))

    @Test
    fun `a survey says how high above the surface, what a shot covers and how often it fires`() {
        val view = stats(
            """{"available":true,"areaText":"9.0 ha","warning":"",
               "surfaceDistanceText":"60.0 m","footprintText":"12.5 × 8.0 m","intervalText":"2.4 s"}""",
        )

        assertEquals(
            "60.0 m above the surface · each shot covers 12.5 × 8.0 m · a shot every 2.4 s",
            cameraText(view),
        )
    }

    @Test
    fun `the core's em dash is an absence, not a figure to print`() {
        val view = stats(
            """{"available":true,"areaText":"—","warning":"",
               "surfaceDistanceText":"—","footprintText":"—","intervalText":"—"}""",
        )

        assertNull(cameraText(view))
        assertEquals("", view!!.areaText)
    }

    @Test
    fun `a survey the core answered partly says the part it knows`() {
        val view = stats(
            """{"available":true,"areaText":"9.0 ha","warning":"",
               "footprintText":"12.5 × 8.0 m"}""",
        )

        assertEquals("each shot covers 12.5 × 8.0 m", cameraText(view))
    }

    @Test
    fun `no survey and no stats say nothing rather than an empty row`() {
        assertNull(cameraText(null))
        assertNull(cameraText(stats("""{"available":false}""")))
    }
}
