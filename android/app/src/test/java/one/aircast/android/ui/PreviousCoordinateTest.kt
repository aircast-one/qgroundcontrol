package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PreviousCoordinateTest {
    @Test
    fun `move to previous item is offered only when the core names a previous position`() {
        assertEquals(47.1 to 8.1, previousCoordinate(JSONObject("""{"previousCoordinate":{"latitude":47.1,"longitude":8.1}}""")))
        assertNull(previousCoordinate(JSONObject("""{"previousCoordinate":null}""")))
        assertNull(previousCoordinate(null))
    }

    @Test
    fun `the altitude hint is the core's sentence or nothing`() {
        assertEquals("Actual AMSL alt sent: 512.3 m", altitudeHint(JSONObject("""{"altitudeHint":"Actual AMSL alt sent: 512.3 m"}""")))
        assertNull(altitudeHint(JSONObject("""{"altitudeHint":null}""")))
    }

    @Test
    fun `a landing pattern offers altitudes relative to launch`() {
        assertEquals(false, altitudesRelative(JSONObject("""{"landing":true,"altitudesAreRelative":false}""")))
        assertNull(altitudesRelative(JSONObject("""{"landing":false,"altitudesAreRelative":null}""")))
    }
}
