package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NotBuiltHereTest {

    private fun fact(name: String, enabled: Boolean = true, readOnly: Boolean = false) = Fact(
        path = "settings.x.$name",
        name = name,
        description = "",
        units = "",
        valueString = "",
        value = null,
        enumStrings = emptyList(),
        enumIndex = -1,
        isBool = true,
        isString = false,
        enabled = enabled,
        readOnly = readOnly,
    )

    @Test
    fun `a control this head does not implement says so instead of accepting a write`() {
        assertEquals(
            "No effect here - this head has no presets tab",
            inertNote(fact("displayPresetsTabFirst")),
        )
    }

    @Test
    fun `the core's own disabled reason still wins over the capability list`() {
        assertEquals(
            "a fact the core disables is disabled for a reason true in every head, and that " +
                "sentence is more specific than ours - enforceChecklist is off because " +
                "useChecklist is off, not because this head cannot draw it",
            "Has no effect while the preflight checklist is off.",
            inertNote(
                fact("enforceChecklist", enabled = false)
                    .copy(disabledReason = "Has no effect while the preflight checklist is off."),
            ),
        )
    }

    @Test
    fun `a control this head does implement is untouched`() {
        assertNull(notBuiltHere(fact("useChecklist")))
        assertNull(notBuiltHere(fact("rcControls")))
        assertNull(notBuiltHere(fact("elevationMapProvider")))
        assertEquals("Read-only", inertNote(fact("useChecklist", readOnly = true)))
    }

    @Test
    fun `the one gimbal setting a joystick can still drive is not claimed to be dead`() {
        assertNull(
            "joystickButtonsSpeed reaches gimbalPitchStart via Joystick.cc, a path needing no " +
                "head UI at all, so it can be live with a joystick attached",
            notBuiltHere(fact("joystickButtonsSpeed")),
        )
    }

    @Test
    fun `every reason reads as a sentence about this head, not about the setting`() {
        NOT_BUILT_HERE.forEach { (name, reason) ->
            assertTrue("$name: reason should not end in a full stop", !reason.endsWith("."))
            assertTrue("$name: reason should be lower case", reason.first().isLowerCase())
        }
    }
}
