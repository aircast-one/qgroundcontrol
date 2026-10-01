package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AutotuneSectionTest {
    @Test
    fun `autotune state and the tuning flight modes read from their views`() {
        assertEquals(
            AutotuneState(false, "Take off first - auto-tuning runs in flight", 0f, "WARNING!"),
            autotuneState(JSONObject("""{"available":true,"canStart":false,"status":"Take off first - auto-tuning runs in flight","progress":0,"warning":"WARNING!"}""")),
        )
        assertNull(autotuneState(JSONObject("""{"available":false}""")))
        assertEquals(TuningModes("Stabilized", "Hold"), tuningModes(JSONObject("""{"stabilizedFlightMode":"Stabilized","pauseFlightMode":"Hold"}""")))
    }
}
