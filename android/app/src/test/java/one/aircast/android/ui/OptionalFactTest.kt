package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class OptionalFactTest {
    @Test
    fun `an optional field reads switched off while it holds no number`() {
        val off = factFromControl(JSONObject("""{"name":"Yaw","path":"p","optional":true,"value":null,"valueText":""}"""))!!
        assertEquals(true, off.optional)
        assertEquals(false, off.optionalSet)
        val on = factFromControl(JSONObject("""{"name":"Yaw","path":"p","optional":true,"value":15.0,"valueText":"15.0"}"""))!!
        assertEquals(true, on.optionalSet)
    }
}
