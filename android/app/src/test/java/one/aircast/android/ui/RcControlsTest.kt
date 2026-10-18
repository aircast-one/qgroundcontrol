package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RcControlsTest {
    @Test
    fun `each control reads the position the core reports for its channel`() {
        val view = org.json.JSONObject(
            """{"controls":[
                {"label":"Mode","channel":5,"type":"switch3","pwm":1985,"position":2,"on":true,"held":false},
                {"label":"CH7","channel":7,"type":"button","pwm":null,"position":null,"on":null,"held":true},
                {"label":"Odd","channel":9,"type":"dial","pwm":1500,"position":1,"on":false,"held":false}]}""",
        )

        assertEquals(
            listOf(
                RcControlState(RcControl("Mode", 5, RcControlType.Switch3), pwm = 1985, position = 2, on = true, held = false),
                RcControlState(RcControl("CH7", 7, RcControlType.Button), pwm = null, position = null, on = null, held = true),
            ),
            parseRcControlStates(view),
        )
        assertEquals(emptyList<RcControlState>(), parseRcControlStates(null))
    }

    @Test
    fun `an unset setting yields no controls`() {
        assertEquals(emptyList<RcControl>(), parseRcControls("[]"))
        assertEquals(emptyList<RcControl>(), parseRcControls(null))
        assertEquals(emptyList<RcControl>(), parseRcControls(""))
    }

    @Test
    fun `each supported type is read`() {
        val parsed = parseRcControls(
            """[{"label":"Gimbal","channel":7,"type":"slider"},
                {"label":"Light","channel":8,"type":"button"},
                {"label":"Mode","channel":9,"type":"switch3"},
                {"label":"Drop","channel":10,"type":"momentary"}]""",
        )

        assertEquals(
            listOf(
                RcControl("Gimbal", 7, RcControlType.Slider),
                RcControl("Light", 8, RcControlType.Button),
                RcControl("Mode", 9, RcControlType.Switch3),
                RcControl("Drop", 10, RcControlType.Momentary),
            ),
            parsed,
        )
    }

    @Test
    fun `a control with no channel is dropped rather than sent to channel zero`() {
        assertEquals(emptyList<RcControl>(), parseRcControls("""[{"label":"X","type":"slider"}]"""))
    }

    @Test
    fun `an unknown type is dropped rather than guessed`() {
        assertEquals(
            emptyList<RcControl>(),
            parseRcControls("""[{"label":"X","channel":5,"type":"dial"}]"""),
        )
    }

    @Test
    fun `a control with no label falls back to its channel`() {
        assertEquals(
            listOf(RcControl("CH6", 6, RcControlType.Slider)),
            parseRcControls("""[{"channel":6,"type":"slider"}]"""),
        )
    }

    @Test
    fun `malformed json yields nothing rather than throwing`() {
        assertEquals(emptyList<RcControl>(), parseRcControls("not json"))
    }
}

class RcSendRateTest {
    @Test
    fun `a drag does not send on every pixel`() {
        assertFalse(rcSendDue(nowMs = 1_040, lastSentMs = 1_000, finished = false))
        assertFalse(rcSendDue(nowMs = 1_099, lastSentMs = 1_000, finished = false))
    }

    @Test
    fun `a drag still sends often enough to feel live`() {
        assertTrue(rcSendDue(nowMs = 1_100, lastSentMs = 1_000, finished = false))
        assertTrue(rcSendDue(nowMs = 2_000, lastSentMs = 1_000, finished = false))
    }

    @Test
    fun `letting go always sends, so the vehicle ends where the finger did`() {
        assertTrue(rcSendDue(nowMs = 1_001, lastSentMs = 1_000, finished = true))
        assertTrue(rcSendDue(nowMs = 1_000, lastSentMs = 1_000, finished = true))
    }

    @Test
    fun `the first send of a drag is never throttled`() {
        assertTrue(rcSendDue(nowMs = 5_000, lastSentMs = 0, finished = false))
    }
}
