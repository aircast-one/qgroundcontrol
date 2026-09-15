package one.aircast.android

import androidx.compose.ui.text.input.KeyboardType
import one.aircast.android.bridge.Fact
import one.aircast.android.ui.factKeyboard
import one.aircast.android.ui.factValueLines
import one.aircast.android.ui.typedValue
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

    @Test
    fun `a text value is readable without entering the control that writes it`() {
        assertEquals(
            "a comma list overflows a single-line field - Flight Modes shows six rows of " +
                "Acro,Circle,Drift,Sport,Flip,Bra... - and reading the rest means tapping into an " +
                "editable field, which on a phone also raises the keyboard over the page",
            4,
            factValueLines(fact("0", isString = true)),
        )
    }

    @Test
    fun `a number keeps one line, because a number does not overflow`() {
        assertEquals(1, factValueLines(fact("200")))
        assertEquals(1, factValueLines(fact("-50")))
        assertEquals(1, factValueLines(fact("0", isBool = true)))
    }

    @Test
    fun `a wrapping field must not let the return key into a settings value`() {
        assertEquals(
            "the multi-line field is what puts Enter on the keyboard, so the newline is a hazard " +
                "this change introduces rather than one it found",
            "Acro,Circle",
            typedValue("Acro,\nCircle"),
        )
    }
}
