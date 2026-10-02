package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class SlideLabelTest {
    @Test
    fun namesTheAction() = assertEquals("Slide to land", slideLabel("Land"))

    @Test
    fun fallsBackWithoutAName() = assertEquals("Slide to confirm", slideLabel(""))
}
