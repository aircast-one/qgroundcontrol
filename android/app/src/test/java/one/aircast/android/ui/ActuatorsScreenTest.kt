package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ActuatorsScreenTest {
    @Test
    fun `bitset and true-if-positive write the raw value as QGC's wrapper facts do`() {
        assertTrue(bitsetChecked(5.0, 2))
        assertFalse(bitsetChecked(5.0, 1))
        assertEquals(7L, bitsetWritten(5.0, 1, true))
        assertEquals(1L, bitsetWritten(5.0, 2, false))
        assertEquals(1500.0, signWritten(-1500.0, true), 0.0)
        assertEquals(-1500.0, signWritten(1500.0, false), 0.0)
    }

    @Test
    fun `groups subgroups and channel configs read from the view`() {
        val read = actuatorOutputs(
            JSONObject(
                """{"class":"ActuatorOutputs","available":true,"showUi":true,"groups":[{"label":"MAIN","enable":null,"groupsVisible":true,"params":[],""" +
                    """"subgroups":[{"label":"MAIN 1-4","primary":null,"params":[],"columns":[{"label":"Function","advanced":false,"visible":true}],""" +
                    """"channels":[{"label":"MAIN 1","configs":[null]}]}]}]}""",
            ),
        )!!
        val subgroup = read.groups.single().subgroups.single()
        assertEquals("Function", subgroup.columns.single().label)
        assertNull(subgroup.channels.single().configs.single())
    }
}

class ActuatorGeometryTest {
    @Test
    fun `fixed geometry cells read as read-only values`() {
        val read = geometry(
            JSONObject(
                """{"title":"Geometry: Tiltrotor","helpUrl":"","groups":[{"label":"Motors","count":null,"params":[],""" +
                    """"channels":[{"label":"Rear Motor (Motor 3)","cells":[{"fixed":true,"label":"Position X","valueString":"-0.7500","advanced":false},null]}]}]}""",
            ),
        )!!
        val channel = read.groups.single().channels.single()
        assertEquals("Rear Motor (Motor 3)", channel.label)
        assertEquals(GeometryCell.Fixed("Position X", "-0.7500", false), channel.cells[0])
        assertNull(channel.cells[1])
    }
}

class ActuatorMixerCellTest {
    @Test
    fun `an axis cell and a rule-hidden cell read from the geometry`() {
        val axis = geometryCell(JSONObject("""{"axis":true,"options":["Custom","Upwards"],"index":1,"params":["CA_MC_R0_AX","CA_MC_R0_AY","CA_MC_R0_AZ"],"hidden":false,"disabled":false,"advanced":true}"""))
        assertEquals(GeometryCell.Axis(listOf("Custom", "Upwards"), 1, listOf("CA_MC_R0_AX", "CA_MC_R0_AY", "CA_MC_R0_AZ"), true, false, false), axis)
        val hidden = geometryCell(JSONObject("""{"fixed":true,"label":"Pitch Torque","valueString":"0.0000","advanced":false,"hidden":true}"""))
        assertTrue(hidden!!.hidden)
    }

    @Test
    fun `the mixer is editable only while sliders are off and no motor assignment runs`() {
        assertTrue(mixerEditable(testing = false, assigning = false))
        assertFalse(mixerEditable(testing = true, assigning = false))
        assertFalse(mixerEditable(testing = false, assigning = true))
    }

    @Test
    fun `a cell with a missing parameter reads as not available`() {
        val missing = geometryCell(JSONObject("""{"unavailable":true,"label":"Roll Torque","advanced":false,"hidden":false,"disabled":false,"channelFunction":201,"param":"CA_SV_CS0_TRQ_R"}"""))
        assertEquals(GeometryCell.Unavailable("Roll Torque", false, false), missing)
    }
}

class MotorAssignmentHeadTest {
    @Test
    fun `the assignment message loses its markup and the highlighted motors read as a set`() {
        assertEquals("a\n\nWarning\nb", plainMessage("a<br /><br /><b>Warning</b><br />b"))
        val state = motorAssignment(JSONObject("""{"multirotor":true,"enabled":true,"active":true,"message":"m","highlighted":[1,3]}"""))
        assertEquals(setOf(1, 3), state.highlighted)
        assertTrue(state.active)
    }
}
