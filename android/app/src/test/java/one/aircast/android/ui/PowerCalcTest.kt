package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PowerCalcTest {
    @Test
    fun `a row's calculator is read and asks the core about its own battery and parameter`() {
        val calculator = powerCalculator(JSONObject("""{"title":"Calculate Voltage Divider","measure":"voltage","batteryIndex":2,"param":"BAT2_V_DIV","button":"Calculate"}"""))!!
        assertEquals("view.powerCalc(voltage,2,BAT2_V_DIV)", powerCalcPath(calculator))
        assertNull(powerCalculator(null))
        assertNull(powerCalculator(JSONObject("""{"title":"x"}""")))
    }
}
