package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ControlKindTest {
    @Test
    fun `the head knows exactly the kinds the core enumerates`() {
        assertEquals(
            setOf("toggle", "choice", "bitmask", "text", "number"),
            KNOWN_CONTROL_KINDS,
        )
    }

    @Test
    fun `every enumerated kind is rendered rather than refused`() {
        KNOWN_CONTROL_KINDS.forEach { assertTrue(it, controlIsUnderstood(it)) }
    }

    @Test
    fun `a kind the core adds later is not given an editable control`() {
        assertFalse(controlIsUnderstood("slider"))
        assertFalse(controlIsUnderstood("colour"))
    }

    @Test
    fun `a control with no kind at all is still editable, because that is every other path`() {
        assertTrue(controlIsUnderstood(""))
    }

    @Test
    fun `the kind is carried from the projection onto the fact`() {
        val control = JSONObject(
            """{"label":"Arm Checks","name":"ARMING_CHECK","path":"p","control":"bitmask",
                "value":82,"valueString":"82"}""",
        )

        assertEquals("bitmask", factFromControl(control)?.controlKind)
    }
}
