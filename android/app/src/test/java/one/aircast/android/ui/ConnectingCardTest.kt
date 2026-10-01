package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ConnectingCardTest {
    @Test
    fun namesTheVehicleWhenKnown() = assertEquals("Connecting to guta-test", connectingTitle("guta-test"))

    @Test
    fun fallsBackWithoutAName() {
        assertEquals("Connecting", connectingTitle(null))
        assertEquals("Connecting", connectingTitle(""))
    }
}
