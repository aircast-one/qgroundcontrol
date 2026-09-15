package one.aircast.android

import androidx.compose.ui.text.input.KeyboardType
import one.aircast.android.bridge.Fact
import one.aircast.android.ui.factKeyboard
import one.aircast.android.ui.factValueLines
import one.aircast.android.ui.truncationRefusal
import one.aircast.android.ui.typedValue
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FactKeyboardTest {
    private fun fact(
        min: String,
        isString: Boolean = false,
        isBool: Boolean = false,
        whole: Boolean = false,
    ) = Fact(
        path = "p", name = "n", description = "", units = "", valueString = "0",
        value = 0, enumStrings = emptyList(), enumIndex = -1,
        isBool = isBool, isString = isString, readOnly = false, minString = min,
        wholeNumbersOnly = whole,
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

    @Test
    fun `a fraction typed into a whole-number setting is refused, not silently truncated`() {
        assertEquals(
            "FactMetaData::convertAndValidateRaw does QVariant(3.7).toInt() for an int fact and " +
                "reports convertOk, and setRawValue passes convertOnly so the range check never " +
                "runs - Fact.validate accepts 3.7, the vehicle gets 3, and nothing says so",
            "This setting takes whole numbers only.",
            truncationRefusal(fact("0", whole = true), "3.7"),
        )
    }

    @Test
    fun `a whole number in a whole-number setting passes`() {
        assertNull(truncationRefusal(fact("0", whole = true), "4"))
        assertNull(truncationRefusal(fact("0", whole = true), " 12 "))
        assertNull(truncationRefusal(fact("-5", whole = true), "-3"))
    }

    @Test
    fun `a real-typed setting still takes fractions`() {
        assertNull(
            "decimalPlaces is a different question - a real fact declaring zero decimals is what " +
                "you write for a percentage, and refusing fractions there would reject values the " +
                "vehicle accepts",
            truncationRefusal(fact("0"), "3.7"),
        )
    }

    @Test
    fun `text that is not a number is left to the vehicle's own validator`() {
        assertNull(truncationRefusal(fact("0", whole = true), "not a number"))
        assertNull(truncationRefusal(fact("0", whole = true), ""))
    }

    @Test
    fun `a whole-number setting that cannot go negative gets a keypad with no decimal point`() {
        assertEquals(KeyboardType.Number, factKeyboard(fact("0", whole = true)))
        assertEquals(KeyboardType.Text, factKeyboard(fact("-5", whole = true)))
        assertEquals(KeyboardType.Decimal, factKeyboard(fact("0")))
    }
}
