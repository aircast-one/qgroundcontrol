package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class InstrumentsTest {

    private fun view(json: String) = JSONObject(json)

    @Test
    fun `a reading joins its units and a unitless one does not`() {
        val shown = instruments(
            view("""{"items":[
                {"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},
                {"label":"Heading","value":"356","units":"","missing":false}
            ]}"""),
        )

        assertEquals(listOf("25.0 m", "356"), shown.map { it.reading })
        assertEquals(listOf("Alt (Rel)", "Heading"), shown.map { it.label })
    }

    @Test
    fun `a missing instrument is left out rather than shown blank`() {
        val shown = instruments(
            view("""{"items":[
                {"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},
                {"label":"Distance to Home","value":"","units":"","missing":true}
            ]}"""),
        )

        assertEquals(listOf("Alt (Rel)"), shown.map { it.label })
    }

    @Test
    fun `no view at all is an empty row, not a crash`() {
        assertEquals(emptyList<Instrument>(), instruments(null))
        assertEquals(emptyList<Instrument>(), instruments(view("{}")))
    }
}
