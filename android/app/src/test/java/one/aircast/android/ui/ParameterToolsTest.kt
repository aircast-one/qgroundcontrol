package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ParameterToolsTest {
    @Test
    fun `tools come from the core in the editor's order`() {
        val tools = parameterTools(
            JSONObject(
                """{"tools":[{"path":"parameterTools.refresh","label":"Refresh","confirm":false,"confirmTitle":"","confirmMessage":""},""" +
                    """{"path":"parameterTools.resetDefaults","label":"Reset all to firmware's defaults","confirm":true,"confirmTitle":"Reset All","confirmMessage":"m"},""" +
                    """{"path":"vehicle.rebootVehicle","label":"Reboot Vehicle","confirm":true,"confirmTitle":"Reboot Vehicle","confirmMessage":"Select Ok to reboot vehicle."}]}""",
            ),
        )
        assertEquals(listOf("Refresh", "Reset all to firmware's defaults", "Reboot Vehicle"), tools.map { it.label })
        assertFalse(tools[0].confirm)
        assertEquals("Reset", confirmLabel(tools[1]))
        assertEquals("Ok", confirmLabel(tools[2]))
    }

    @Test
    fun `the modified filter keeps only parameters changed from stock`() {
        assertTrue(parameterShown("RTL_ALT", "", "", modifiedOnly = true, modified = setOf("RTL_ALT")))
        assertFalse(parameterShown("RTL_SPEED", "", "", modifiedOnly = true, modified = setOf("RTL_ALT")))
        assertTrue(parameterShown("RTL_SPEED", "", "rtl", modifiedOnly = false, modified = emptySet()))
    }
}
