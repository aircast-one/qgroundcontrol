package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class InstrumentStyleTest {
    @Test
    fun `the instrument style follows the qml file the setting names`() {
        assertEquals(InstrumentStyle.Horizontal, instrumentStyle("qrc:/qml/QGroundControl/FlightMap/Widgets/HorizontalCompassAttitude.qml"))
        assertEquals(InstrumentStyle.Vertical, instrumentStyle("qrc:/qml/QGroundControl/FlightMap/Widgets/VerticalCompassAttitude.qml"))
        assertEquals(InstrumentStyle.Integrated, instrumentStyle("qrc:/qml/QGroundControl/FlightMap/Widgets/IntegratedCompassAttitude.qml"))
        assertEquals(InstrumentStyle.Integrated, instrumentStyle(null))
    }
}
