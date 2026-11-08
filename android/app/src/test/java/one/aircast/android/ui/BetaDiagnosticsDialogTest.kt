package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class BetaDiagnosticsDialogTest {
    @Test
    fun `the offer shows while the core says a beta build has not answered`() {
        val prompt = betaDiagnostics(
            JSONObject(
                """{"show":true,"title":"Help us debug this beta","text":"Share logs?","accept":"Share logs","dismiss":"Not now","endpoint":"otel.aircast.one:443"}""",
            ),
        )!!
        assertEquals("Help us debug this beta", prompt.title)
        assertEquals("Share logs", prompt.accept)
        assertEquals("Not now", prompt.dismiss)
        assertEquals("otel.aircast.one:443", prompt.endpoint)
        assertNull(betaDiagnostics(JSONObject("""{"show":false}""")))
        assertNull(betaDiagnostics(null))
    }
}
