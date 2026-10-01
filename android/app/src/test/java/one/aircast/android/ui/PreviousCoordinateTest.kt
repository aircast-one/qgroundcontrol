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
}
