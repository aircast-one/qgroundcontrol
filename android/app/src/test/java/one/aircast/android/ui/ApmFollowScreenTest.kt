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
    fun `the lead list carries each system id so a pick names the system rather than a row`() {
        val follow = apmFollow(
            JSONObject("""{"available":true,"leads":[{"id":255,"name":"Ground station"},{"id":2,"name":"Vehicle 2"}],"leadIndex":1}"""),
        )!!
        assertEquals(listOf(FollowLead(255, "Ground station"), FollowLead(2, "Vehicle 2")), follow.leads)
        assertEquals(2, follow.leads[follow.leadIndex].id)
        assertEquals(-1, apmFollow(JSONObject("""{"available":true}"""))!!.leadIndex)
    }

    @Test
    fun `offsets show and type in the app distance units`() {
        val follow = apmFollow(
            JSONObject(
                """{"available":true,"distance":3.048,"height":6.096,"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,
                "verticalUnit":"ft","verticalMetresPerUnit":0.3048}""",
            ),
        )!!
        assertEquals("10.0 ft", follow.horizontal.text(follow.distance))
        assertEquals("20.0 ft", follow.vertical.text(follow.height))
                assertEquals("5.0 m", apmFollow(JSONObject("""{"available":true,"distance":5.0}"""))!!.horizontal.text(5.0))
    }

    @Test
    fun `a tap sets the heading of the vehicle around the ground station`() {
        org.junit.Assert.assertEquals(0.0, headingOfTap(0f, 10f), 1e-9)
        org.junit.Assert.assertEquals(90.0, headingOfTap(10f, 0f), 1e-9)
        org.junit.Assert.assertEquals(180.0, headingOfTap(0f, -10f), 1e-9)
        org.junit.Assert.assertEquals(270.0, headingOfTap(-10f, 0f), 1e-9)
    }
}
