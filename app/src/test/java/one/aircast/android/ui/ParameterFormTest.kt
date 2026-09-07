package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ParameterFormTest {
    @Test
    fun `a fact response becomes a fact addressed by its parameter path`() {
        val json = JSONObject(
            """
            {"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude",
             "units":"cm","valueString":"1500","value":1500,"typeIsBool":false,"readOnly":false}
            """.trimIndent(),
        )
        val fact = factFromParameter("RTL_ALT", json)!!
        assertEquals(parameterPath("RTL_ALT"), fact.path)
        assertEquals("RTL_ALT", fact.name)
        assertEquals("Return altitude", fact.description)
        assertEquals("cm", fact.units)
    }

    @Test
    fun `a parameter with no description is labelled by its name`() {
        val json = JSONObject("""{"kind":"fact","name":"","shortDescription":"","valueString":"0"}""")
        val fact = factFromParameter("MNT_ANGMIN_PAN", json)!!
        assertEquals("MNT_ANGMIN_PAN", fact.name)
        assertEquals("MNT_ANGMIN_PAN", fact.title)
    }

    @Test
    fun `a description still wins over the name`() {
        val json = JSONObject("""{"kind":"fact","name":"RTL_ALT","shortDescription":"Return altitude"}""")
        assertEquals("Return altitude", factFromParameter("RTL_ALT", json)!!.title)
    }

    @Test
    fun `a parameter the vehicle does not have is skipped`() {
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"value","value":null}""")))
        assertNull(factFromParameter("NOPE", JSONObject("""{"kind":"null"}""")))
        assertNull(factFromParameter("NOPE", JSONObject("{}")))
    }

    @Test
    fun `safety sections are chosen by firmware`() {
        val apm = setupSectionsFor("Safety", isPx4 = false)!!
        val px4 = setupSectionsFor("Safety", isPx4 = true)!!
        assertEquals(true, apm.any { it.names.contains("FS_THR_ENABLE") })
        assertEquals(true, px4.any { it.names.contains("NAV_RCL_ACT") })
        assertEquals(false, apm.any { it.names.contains("NAV_RCL_ACT") })
    }

    @Test
    fun `camera and lights are ardupilot only, flight behavior is px4 only`() {
        assertEquals(null, setupSectionsFor("Camera", isPx4 = true))
        assertEquals(null, setupSectionsFor("Lights", isPx4 = true))
        assertEquals(null, setupSectionsFor("Flight Behavior", isPx4 = false))
        assertEquals(true, setupSectionsFor("Camera", isPx4 = false)!!.isNotEmpty())
        assertEquals(true, setupSectionsFor("Lights", isPx4 = false)!!.isNotEmpty())
        assertEquals(true, setupSectionsFor("Flight Behavior", isPx4 = true)!!.isNotEmpty())
    }

    @Test
    fun `light channels cover every servo output the page offers`() {
        val channels = setupSectionsFor("Lights", isPx4 = false)!!.first().names
        assertEquals(12, channels.size)
        assertEquals("SERVO5_FUNCTION", channels.first())
        assertEquals("SERVO16_FUNCTION", channels.last())
    }

    @Test
    fun `flight modes offer six slots on both firmwares`() {
        val apm = setupSectionsFor("Flight Modes", isPx4 = false)!!
        val px4 = setupSectionsFor("Flight Modes", isPx4 = true)!!
        assertEquals(listOf("FLTMODE1", "FLTMODE6"), apm[1].names.let { listOf(it.first(), it.last()) })
        assertEquals(6, apm[1].names.size)
        assertEquals(listOf("COM_FLTMODE1", "COM_FLTMODE6"), px4[1].names.let { listOf(it.first(), it.last()) })
        assertEquals("FLTMODE_CH", apm[0].names.single())
        assertEquals("RC_MAP_FLTMODE", px4[0].names.single())
    }

    @Test
    fun `power covers both batteries on ardupilot and both naming eras on px4`() {
        val apm = setupSectionsFor("Power", isPx4 = false)!!
        val px4 = setupSectionsFor("Power", isPx4 = true)!!
        assertEquals(true, apm.any { it.names.contains("BATT_MONITOR") })
        assertEquals(true, apm.any { it.names.contains("BATT2_MONITOR") })
        assertEquals(true, px4.first().names.containsAll(listOf("BAT_N_CELLS", "BAT1_N_CELLS")))
        assertEquals(false, apm.any { it.names.contains("BAT_N_CELLS") })
    }

    @Test
    fun `tuning is ardupilot only and warns on the rate gains`() {
        assertNull(setupSectionsFor("Tuning", isPx4 = true))
        val apm = setupSectionsFor("Tuning", isPx4 = false)!!
        val rates = apm.first { it.title == "Rate gains" }
        assertEquals(true, rates.names.contains("ATC_RAT_RLL_P"))
        assertEquals(true, rates.note.contains("small steps"))
    }

    @Test
    fun `frame is the two ardupilot frame parameters and warns about motor order`() {
        assertNull(setupSectionsFor("Frame", isPx4 = true))
        val frame = setupSectionsFor("Frame", isPx4 = false)!!.single()
        assertEquals(listOf("FRAME_CLASS", "FRAME_TYPE"), frame.names)
        assertEquals(true, frame.note.contains("motor order"))
    }

    @Test
    fun `components without a native form report none`() {
        assertNull(setupSectionsFor("Sensors", isPx4 = false))
        assertNull(setupSectionsFor("Radio", isPx4 = true))
    }

    @Test
    fun `an empty error from QGC means the value is acceptable`() {
        assertNull(validationMessage(""))
        assertNull(validationMessage("   "))
    }

    @Test
    fun `QGC's own wording is passed through unchanged`() {
        assertEquals(
            "Value must be within 0 and 100",
            validationMessage("Value must be within 0 and 100"),
        )
    }

    @Test
    fun `a bridge call that returned nothing does not block the write`() {
        assertNull(validationMessage(null))
    }

    @Test
    fun `a non-string result is not treated as an error`() {
        assertNull(validationMessage(false))
        assertNull(validationMessage(0))
    }
}
