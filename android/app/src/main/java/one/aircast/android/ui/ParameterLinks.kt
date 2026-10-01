package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.fromHtml
import androidx.compose.ui.graphics.Color
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact

private const val PARAM_SCHEME = "param://"

internal fun paramLinkName(url: String): String? = url.takeIf { it.startsWith(PARAM_SCHEME) }?.removePrefix(PARAM_SCHEME)?.takeIf { it.isNotBlank() }

@Composable
internal fun LinkedText(html: String, style: TextStyle, color: Color, onParameter: (String) -> Unit, modifier: Modifier = Modifier) {
    val uris = LocalUriHandler.current
    val text = remember(html) {
        AnnotatedString.fromHtml(html) { link ->
            val url = (link as? LinkAnnotation.Url)?.url ?: return@fromHtml
            paramLinkName(url)?.let(onParameter) ?: runCatching { uris.openUri(url) }
        }
    }
    Text(text, style = style, color = color, modifier = modifier)
}

@Composable
internal fun ParameterEditDialog(name: String, onDismiss: () -> Unit) {
    var revision by remember { mutableIntStateOf(0) }
    val fact by produceState<Fact?>(null, name, revision) {
        value = withContext(Dispatchers.Default) { parameterFact(name) }
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Edit Parameter") },
        text = {
            fact?.let { loaded ->
                FactRow(fact = loaded, title = loaded.name, subtitle = parameterSubtitle(loaded.description, loaded.units), onWrite = { revision++ })
            } ?: Text("$name is not a parameter on this vehicle.", style = MaterialTheme.typography.bodySmall)
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
    )
}
