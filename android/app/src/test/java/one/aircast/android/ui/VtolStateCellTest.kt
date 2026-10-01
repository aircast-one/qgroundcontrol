package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VtolStateCellTest {
    @Test
    fun `a vtol shows its flight state and offers the opposite transition`() {
        val forward = vtolState(JSONObject("""{"vtol":true,"vtolInFwdFlight":true,"flying":true}"""))!!
        assertEquals("FW(vtol)", forward.label)
        assertEquals("vtolTransitionToMultiRotor", forward.transition)
        assertEquals("MR(vtol)", vtolState(JSONObject("""{"vtol":true,"vtolInFwdFlight":false}"""))!!.label)
        assertNull(vtolState(JSONObject("""{"vtol":false}""")))
    }
}
