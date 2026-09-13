package one.aircast.android

import one.aircast.android.ui.plainNumber
import org.junit.Assert.assertEquals
import org.junit.Test

class PlainNumberTest {
    @Test
    fun `a limit in scientific notation is spelled out`() {
        assertEquals("300000", plainNumber("3e+05"))
        assertEquals("-1000", plainNumber("-1e3"))
    }

    @Test
    fun `a whole number loses its decimal tail`() {
        assertEquals("200", plainNumber("200"))
        assertEquals("200", plainNumber("200.000"))
    }

    @Test
    fun `a fractional limit keeps its digits`() {
        assertEquals("0.05", plainNumber("0.05"))
        assertEquals("1.5", plainNumber("1.50"))
    }

    @Test
    fun `a limit too large to spell out is left alone`() {
        assertEquals("3.4e38", plainNumber("3.4e38"))
    }

    @Test
    fun `anything that is not a number passes through untouched`() {
        assertEquals("", plainNumber(""))
        assertEquals("Auto", plainNumber("Auto"))
    }
}
