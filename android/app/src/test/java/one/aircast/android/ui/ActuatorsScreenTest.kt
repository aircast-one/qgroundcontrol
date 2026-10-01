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
