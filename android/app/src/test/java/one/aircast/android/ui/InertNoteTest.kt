package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class InertNoteTest {

    private fun fact(
        name: String,
        enabled: Boolean = true,
        readOnly: Boolean = false,
        isBool: Boolean = true,
    ) = Fact(
        path = "settings.x.$name",
        name = name,
        description = "",
        units = "",
        valueString = "",
        value = null,
        enumStrings = emptyList(),
        enumIndex = -1,
        isBool = isBool,
        isString = false,
        enabled = enabled,
        readOnly = readOnly,
    )

    @Test
    fun `the core's own disabled reason is the note`() {
        assertEquals(
            "Has no effect while the preflight checklist is off.",
            inertNote(
                fact("enforceChecklist", enabled = false)
                    .copy(disabledReason = "Has no effect while the preflight checklist is off."),
            ),
        )
        assertEquals("Has no effect yet", inertNote(fact("enforceChecklist", enabled = false)))
    }

    @Test
    fun `a read-only fact says so`() {
        assertEquals("Read-only", inertNote(fact("useChecklist", readOnly = true)))
    }

    @Test
    fun `every setting is built here, so none is called inert while enabled`() {
        assertEquals("Read-only", inertNote(fact("displayPresetsTabFirst")))
        assertTrue(showsAsField(fact("displayPresetsTabFirst", isBool = false)))
    }

    @Test
    fun `an unknown control kind says edit on desktop`() {
        assertTrue(editOnDesktop(fact("useChecklist").copy(controlKind = "slider")))
    }

    @Test
    fun `a subtitle carries only the units`() {
        assertEquals("MB", factSubtitle(fact("someLiveNumber", isBool = false).copy(units = "MB")))
        assertEquals("", factSubtitle(fact("someLiveToggle")))
    }
}
