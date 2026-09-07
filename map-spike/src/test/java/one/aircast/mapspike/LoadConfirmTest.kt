package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class LoadConfirmTest {
    @Test
    fun `a clean plan loads without asking`() {
        assertEquals(LoadStep.Load, loadStep(dirty = false, armed = false))
    }

    @Test
    fun `unsent changes are not discarded on the first tap`() {
        assertEquals(LoadStep.Confirm, loadStep(dirty = true, armed = false))
    }

    @Test
    fun `the second tap goes through`() {
        assertEquals(LoadStep.Load, loadStep(dirty = true, armed = true))
    }

    @Test
    fun `an armed confirm on a clean plan still just loads`() {
        assertEquals(LoadStep.Load, loadStep(dirty = false, armed = true))
    }
}
