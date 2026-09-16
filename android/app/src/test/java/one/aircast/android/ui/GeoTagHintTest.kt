package one.aircast.android.ui

import org.junit.Assert.assertTrue
import org.junit.Test

class GeoTagHintTest {

    @Test
    fun `the hint names a settings page this head actually has`() {
        assertTrue(
            "the hint used to say \"under MAVLink and telemetry logs\", which is not a page or a " +
                "section - the page is MAVLink and its note begins 'Telemetry logging'. An " +
                "instruction that names a control has to name it as the operator will read it",
            PAGE_NOTES.keys.contains(TELEMETRY_LOG_PAGE),
        )
        assertTrue(noTelemetryLogsText().contains(TELEMETRY_LOG_PAGE))
    }

    @Test
    fun `and quotes the setting by the label QGC gives it`() {
        assertTrue(noTelemetryLogsText().contains("\"$TELEMETRY_LOG_SETTING\""))
    }
}
