package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

class ApmAirframeScreenTest {
    @Test
    fun `classes, their filtered types and the invalid flag come from the core`() {
        val read = apmAirframe(JSONObject("""{"available":true,"help":"Currently set","frameClass":2,"frameType":3,"invalidText":"Invalid setting for FRAME_TYPE. Click to Reset.",
            "classes":[{"name":"Quad","value":1,"chosen":false,"image":"QuadRotorX.svg","types":[{"name":"X","value":1}],"valid":true},
                       {"name":"Hexa","value":2,"chosen":true,"image":"AirframeUnknown.svg","types":[{"name":"X","value":1},{"name":"Plus","value":0}],"valid":false}]}"""))!!
        assertEquals(listOf("Quad", "Hexa"), read.classes.map { it.name })
        assertEquals(3, read.frameType)
        assertFalse(read.classes[1].valid)
        assertEquals(listOf(1, 0), read.classes[1].types.map { it.value })
        assertNull(apmAirframe(JSONObject("""{"available":false}""")))
    }
}
