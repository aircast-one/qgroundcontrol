package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class RcControlsTest {
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
