package one.aircast.android.ui

import one.aircast.android.R
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.Alignment
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Icon
import androidx.compose.material3.Button
import androidx.compose.material3.Surface
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.Dialog
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val FIRST_RUN_PATH = "view.firstRun"

internal data class FirstRun(
    val title: String,
    val vehicleHeading: String,
    val vehicleDescription: String,
    val preferences: List<Fact>,
    val unitsHeading: String,
    val unitsDescription: String,
)

internal fun firstRun(view: JSONObject?): FirstRun? =
    view?.takeIf { it.optBoolean("show") }?.let {
        val listed = it.optJSONArray("vehiclePreferences")
        FirstRun(
            title = it.optText("title"),
            vehicleHeading = it.optText("vehicleHeading"),
            vehicleDescription = it.optText("vehicleDescription"),
            preferences = (0 until (listed?.length() ?: 0)).mapNotNull { index -> listed!!.optJSONObject(index)?.let(::factFromControl) },
            unitsHeading = it.optText("unitsHeading"),
            unitsDescription = it.optText("unitsDescription"),
        )
    }

@Composable
fun FirstRunDialog() {
    val view by qgcPath(FIRST_RUN_PATH)
    val prompt = remember(view) { firstRun(view) } ?: return
    val close = { offMainDetached { Qgc.invoke("firstRun.markShown") } }

    Dialog(onDismissRequest = close, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
            Column(
                Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 8.dp, vertical = 48.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Box(Modifier.size(96.dp).background(MaterialTheme.colorScheme.primaryContainer, CircleShape), contentAlignment = Alignment.Center) {
                        Icon(painterResource(R.drawable.ic_flight), null, tint = MaterialTheme.colorScheme.onPrimaryContainer, modifier = Modifier.size(48.dp))
                    }
                    Text(prompt.title, style = MaterialTheme.typography.titleLarge, textAlign = TextAlign.Center)
                }
                if (prompt.preferences.isNotEmpty()) {
                    SectionHeader(prompt.vehicleHeading)
                    Text(prompt.vehicleDescription, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp))
                    prompt.preferences.forEach { fact -> FactRow(fact) }
                }
                SectionHeader(prompt.unitsHeading)
                Text(prompt.unitsDescription, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(horizontal = 16.dp))
                UnitsSection(Modifier.fillMaxWidth())
                Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.End) {
                    Button(onClick = close) { Text("Continue") }
                }
            }
        }
    }
}
