package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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

    AlertDialog(
        onDismissRequest = close,
        title = { Text(prompt.title) },
        text = {
            Column(Modifier.heightIn(max = 520.dp).verticalScroll(rememberScrollState())) {
                if (prompt.preferences.isNotEmpty()) {
                    Text(prompt.vehicleHeading, style = MaterialTheme.typography.titleSmall)
                    Text(prompt.vehicleDescription, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(bottom = 8.dp))
                    prompt.preferences.forEach { fact -> FactRow(fact) }
                }
                Text(prompt.unitsHeading, style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 12.dp))
                Text(prompt.unitsDescription, style = MaterialTheme.typography.bodySmall)
                UnitsSection(Modifier.fillMaxWidth())
            }
        },
        confirmButton = { TextButton(onClick = close) { Text("Done") } },
    )
}
