package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PresetNameTest {
    @Test
    fun `the name is checked as it is typed, like TransectStyleComplexItemEditor`() {
        assertEquals("Preset name cannot be blank.", presetNameError("  "))
        assertEquals("Preset name cannot include the \"/\" character.", presetNameError("a/b"))
        assertNull(presetNameError("Farm 2"))
    }
}
