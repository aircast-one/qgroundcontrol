package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OperatorControlTest {

    private fun served(body: String) = JSONObject("""{"kind":"object","class":"OperatorControl",$body}""")

    private val unavailable = served(
        """"available":false,"known":false,"inControl":null,"holderSystemId":null,
           "takeoverAllowed":null,"systemManager":null,"reason":"No vehicle is connected." """,
    )

    private val silent = served(
        """"available":true,"known":false,"inControl":null,"holderSystemId":null,"takeoverAllowed":null,
           "requestAllowed":true,"reason":"This vehicle has not said who is flying it." """,
    )

    private fun elsewhere(takeover: Boolean, request: Boolean = true) = served(
        """"available":true,"known":true,"inControl":false,"holderSystemId":42,"takeoverAllowed":$takeover,
           "requestAllowed":$request,"reason":"Another ground station is flying this vehicle." """,
    )

    private val ours = served(
        """"available":true,"known":true,"inControl":true,"holderSystemId":255,"takeoverAllowed":true,
           "requestAllowed":true,"reason":"" """,
    )

    @Test
    fun `a vehicle that is not there is not a station question`() {
        assertNull(controlStation(unavailable))
        assertNull(controlStation(null))
    }

    @Test
    fun `a vehicle that has never said who is flying it is not reported as taken`() {
        val station = controlStation(silent)!!
        assertFalse(station.known)
        assertNull(station.inControl)
        assertFalse(controlIsElsewhere(station))
        assertNull(
            "a single-station setup would carry this sentence forever, and QGC hides the indicator entirely until the vehicle answers",
            controlLine(station),
        )
    }

    @Test
    fun `holding control offers no request, only the takeover condition`() {
        val station = controlStation(ours)!!
        assertFalse(controlIsElsewhere(station))
        assertNull(controlLine(station))
        assertNull(acquireLabel(station))
        assertEquals("System in control: This GCS (255)", inControlLine(station))
        assertEquals("Takeover allowed", takeoverLine(station))
        assertFalse("Change is off until the box differs from what the vehicle reports", takeoverChangeable(station, true))
        assertTrue(takeoverChangeable(station, false))
        assertFalse(takeoverChangeable(controlStation(elsewhere(takeover = true)), false))
        assertTrue("QGC enables the box while this station holds control", allowTakeoverEditable(station))
        assertTrue("or while the holder allows a takeover", allowTakeoverEditable(controlStation(elsewhere(takeover = true))))
        assertFalse(allowTakeoverEditable(controlStation(elsewhere(takeover = false))))
        assertTrue("QGC enables the box while this station holds control", allowTakeoverEditable(station))
        assertTrue("or while the holder allows a takeover", allowTakeoverEditable(controlStation(elsewhere(takeover = true))))
        assertFalse(allowTakeoverEditable(controlStation(elsewhere(takeover = false))))
        assertTrue("QGC enables the box while this station holds control", allowTakeoverEditable(station))
        assertTrue("or while the holder allows a takeover", allowTakeoverEditable(controlStation(elsewhere(takeover = true))))
        assertFalse(allowTakeoverEditable(controlStation(elsewhere(takeover = false))))
    }

    @Test
    fun `the takeover condition shows whoever holds control, and the request counts down`() {
        assertEquals("Takeover NOT allowed", takeoverLine(controlStation(elsewhere(takeover = false))))
        assertNull(takeoverLine(controlStation(silent)))
        assertNull(inControlLine(controlStation(elsewhere(takeover = true))))
        assertEquals("Request sent: 9.5", requestSentLabel(9_500))
        assertEquals("Request sent: 0.0", requestSentLabel(-20))
    }

    @Test
    fun `another station is named by the system it flies under`() {
        val station = controlStation(elsewhere(takeover = true))!!
        assertTrue(controlIsElsewhere(station))
        assertEquals("Another ground station is flying this vehicle. (system 42)", controlLine(station))
    }

    @Test
    fun `the offer follows whether takeover was allowed`() {
        assertEquals("Acquire control", acquireLabel(controlStation(elsewhere(takeover = true))))
        assertEquals(
            "Ask the other station for control",
            acquireLabel(controlStation(elsewhere(takeover = false))),
        )
    }

    @Test
    fun `a vehicle that refuses control requests is not offered one`() {
        assertNull(acquireLabel(controlStation(elsewhere(takeover = true, request = false))))
    }

    @Test
    fun `a station that never answered is not offered a request either`() {
        assertNull(acquireLabel(controlStation(silent)))
    }

    @Test
    fun `the timeout is only spent when takeover has to be asked for`() {
        assertEquals(
            "Vehicle::requestOperatorControl clamps anything outside 3..60 to the setting's default, so this 0 is what the API takes and never what the command carries - it means do not start a waiting timer, which is what the QML does with it too",
            0,
            requestTimeoutSeconds(controlStation(elsewhere(takeover = true))!!, 10),
        )
        assertEquals(10, requestTimeoutSeconds(controlStation(elsewhere(takeover = false))!!, 10))
    }

    @Test
    fun `an outstanding request is named while the other station has not answered`() {
        val waiting = controlStation(elsewhere(takeover = false, request = false))
        assertEquals("Waiting for the other station to answer", controlWaitLine(waiting))
    }

    @Test
    fun `a station that may still be asked is not described as waiting`() {
        assertNull(controlWaitLine(controlStation(elsewhere(takeover = false))))
    }

    @Test
    fun `the vehicle we fly is never described as waiting on us`() {
        assertNull(controlWaitLine(controlStation(ours)))
        assertNull(controlWaitLine(controlStation(silent)))
    }

    @Test
    fun `the button is withdrawn for exactly as long as the wait line stands`() {
        val waiting = controlStation(elsewhere(takeover = true, request = false))
        assertNull(acquireLabel(waiting))
        assertEquals("Waiting for the other station to answer", controlWaitLine(waiting))
    }
}
