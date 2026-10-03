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
        assertTrue(parameterShown("RTL_ALT", emptyList(), "", modifiedOnly = true, modified = setOf("RTL_ALT")))
        assertFalse(parameterShown("RTL_SPEED", emptyList(), "", modifiedOnly = true, modified = setOf("RTL_ALT")))
        assertTrue(parameterShown("RTL_SPEED", emptyList(), "rtl", modifiedOnly = false, modified = emptySet()))
    }

    @Test
    fun `the modified filter only applies to PX4 like ParameterEditor`() {
        assertTrue(modifiedFilterOn(chosen = true, px4 = true))
        assertFalse(modifiedFilterOn(chosen = true, px4 = false))
        assertFalse(modifiedFilterOn(chosen = false, px4 = true))
    }

    @Test
    fun `a review lists the rows and warns about another vehicle`() {
        val review = parameterReview(
            JSONObject(
                """{"otherVehicle":true,"multipleComponents":false,"rows":[""" +
                    """{"name":"RTL_ALT","fileValue":"2000","vehicleValue":"1500","units":"cm","cannotSend":false},""" +
                    """{"name":"MP_ONLY","fileValue":"4","vehicleValue":"","units":"","cannotSend":true}]}""",
            ),
        )!!
        assertEquals(listOf("RTL_ALT", "MP_ONLY"), review.rows.map { it.name })
        assertEquals(listOf("The parameters in the file are from a different vehicle."), reviewWarnings(review))
        assertEquals("Vehicle 1500 · File 2000 · cm", diffLine(review.rows[0]))
        assertTrue(diffLine(review.rows[1]).contains("cannot send"))
    }

    @Test
    fun `a file row is keyed by component and a parameter new to the vehicle says so`() {
        val gimbal = ParameterDiffRow(org.json.JSONObject(), "MNT_TYPE", "1", "", "", cannotSend = false, componentId = 154, noVehicleValue = true)
        val autopilot = gimbal.copy(componentId = 1, noVehicleValue = false, vehicleValue = "0")
        assertTrue(gimbal.key != autopilot.key)
        assertEquals("Vehicle N/A — new to Vehicle · File 1", diffLine(gimbal))
    }

    @Test
    fun `the count line reads like the Penpot header`() {
        org.junit.Assert.assertEquals("1,204 parameters \u00b7 3 changed", parameterCountLine(1204, 3))
        org.junit.Assert.assertEquals("1 parameter", parameterCountLine(1, 0))
    }
}
