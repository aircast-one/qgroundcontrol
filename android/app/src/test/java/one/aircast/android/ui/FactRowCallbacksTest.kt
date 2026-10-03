package one.aircast.android.ui

import kotlin.metadata.jvm.KotlinClassMetadata
import org.junit.Assert.assertEquals
import org.junit.Test

class FactRowCallbacksTest {
    private fun parameters(facade: String, function: String): List<String> {
        val metadata = Class.forName(facade).getAnnotation(Metadata::class.java)
        val pkg = (KotlinClassMetadata.readStrict(metadata) as KotlinClassMetadata.FileFacade).kmPackage
        return pkg.functions.first { it.name == function }.valueParameters.map { it.name }
    }

    @Test
    fun `a trailing lambda passed to FactRow runs after an accepted write, so callers reload`() {
        assertEquals("onWrite", parameters("one.aircast.android.ui.SettingsScreenKt", "FactRow").last())
    }
}
