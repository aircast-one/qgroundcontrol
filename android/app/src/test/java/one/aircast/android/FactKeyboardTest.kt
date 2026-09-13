package one.aircast.android

import androidx.compose.ui.text.input.KeyboardType
import one.aircast.android.bridge.Fact
import one.aircast.android.ui.factKeyboard
import org.junit.Assert.assertEquals
import org.junit.Test

class FactKeyboardTest {
    private fun fact(
        min: String,
        isString: Boolean = false,
        isBool: Boolean = false,
    ) = Fact(
        path = "p", name = "n", description = "", units = "", valueString = "0",
        value = 0, enumStrings = emptyList(), enumIndex = -1,
        isBool = isBool, isString = isString, readOnly = false, minString = min,
    )

    @Test
    fun `a parameter that cannot go negative gets the number pad`() {
        assertEquals(KeyboardType.Decimal, factKeyboard(fact("200")))
        assertEquals(KeyboardType.Decimal, factKeyboard(fact("0")))
    }

    @Test
    fun `a parameter that can go negative keeps the full keyboard`() {
        assertEquals(KeyboardType.Text, factKeyboard(fact("-50")))
        assertEquals(KeyboardType.Text, factKeyboard(fact("-3.4e38")))
    }

    @Test
    fun `an unstated minimum keeps the full keyboard rather than making a minus untypable`() {
        assertEquals(KeyboardType.Text, factKeyboard(fact("")))
        assertEquals(KeyboardType.Text, factKeyboard(fact("not a number")))
    }

    @Test
    fun `text and boolean facts are never given a number pad`() {
        assertEquals(KeyboardType.Text, factKeyboard(fact("0", isString = true)))
        assertEquals(KeyboardType.Text, factKeyboard(fact("0", isBool = true)))
    }
}
