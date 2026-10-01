package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApmFollowScreenTest {
    @Test
    fun a_vehicle_without_follow_parameters_has_no_page_and_offsets_read_with_one_decimal() {
        assertNull(apmFollow(JSONObject("""{"available":false}""")))
        val follow = apmFollow(
            JSONObject(
                """{"available":true,"enabled":true,"waiting":false,"supported":true,"showSettings":true,"rover":false,
                "positionOptions":["Maintain Current Offsets","Specify Offsets"],"positionIndex":1,
                "pointOptions":["a","b","c"],"pointIndex":-1,"angle":45.0,"distance":5.0,"height":5.0}""",
            ),
        )!!
        assertTrue(follow.showSettings)
        assertEquals(-1, follow.pointIndex)
        assertEquals("Specify Offsets", follow.positionOptions[follow.positionIndex])
        assertEquals("45.0", oneDecimal(follow.angle))
    }

    @Test
    fun `a tap sets the heading of the vehicle around the ground station`() {
        org.junit.Assert.assertEquals(0.0, headingOfTap(0f, 10f), 1e-9)
        org.junit.Assert.assertEquals(90.0, headingOfTap(10f, 0f), 1e-9)
        org.junit.Assert.assertEquals(180.0, headingOfTap(0f, -10f), 1e-9)
        org.junit.Assert.assertEquals(270.0, headingOfTap(-10f, 0f), 1e-9)
    }
}
