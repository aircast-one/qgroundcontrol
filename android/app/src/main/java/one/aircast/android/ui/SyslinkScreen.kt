package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val SYSLINK_SCREEN = "syslink"
private const val SYSLINK_VIEW = "view.syslink"
private const val SYSLINK_POLL_MS = 1000L
private val HEX = Regex("^[0-9A-Fa-f]{0,10}$")

internal data class Syslink(
    val channel: Int,
    val channelHint: String,
    val address: String,
    val addressHint: String,
    val rate: Int,
    val rates: List<String>,
)

internal fun syslink(view: JSONObject?): Syslink? = view?.takeIf { it.optBoolean("available") }?.let {
    val rates = it.optJSONArray("rates")
    Syslink(
        channel = it.optInt("channel"),
        channelHint = it.optText("channelHint"),
        address = it.optText("address"),
        addressHint = it.optText("addressHint"),
        rate = it.optInt("rate", -1),
        rates = (0 until (rates?.length() ?: 0)).map { i -> rates!!.optString(i) },
    )
}

internal fun hexAddress(typed: String): Boolean = HEX.matches(typed)

@Composable
fun SyslinkScreen(modifier: Modifier = Modifier) {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<Syslink?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { syslink(Qgc.get(SYSLINK_VIEW)) }
        delay(SYSLINK_POLL_MS)
        revision++
    }
    fun act(path: String, vararg args: Any) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            read = withContext(Dispatchers.Default) { syslink(Qgc.get(SYSLINK_VIEW)) }
        }
    }
    val radio = read ?: run {
        Text("This vehicle has no Syslink radio.", modifier.padding(16.dp))
        return
    }
    Column(modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SectionHeader("Radio Settings")
        RadioField("Channel", radio.channel.toString(), radio.channelHint, KeyboardType.Number, accept = { typed -> typed.all(Char::isDigit) }) { typed ->
            typed.toIntOrNull()?.let { act("syslink.setChannel", it) }
        }
        RadioField("Address", radio.address, radio.addressHint, KeyboardType.Ascii, accept = ::hexAddress) { act("syslink.setAddress", it) }
        ChoiceField("Data Rate", radio.rates.getOrElse(radio.rate) { "" }, radio.rates) { act("syslink.setRate", it) }
        refusal?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        OutlinedButton(onClick = { act("syslink.resetDefaults") }) { Text("Restore Defaults") }
    }
}

@Composable
private fun RadioField(label: String, value: String, hint: String, keyboard: KeyboardType, accept: (String) -> Boolean, onDone: (String) -> Unit) {
    var typed by remember(value) { mutableStateOf(value) }
    Column {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f))
            OutlinedTextField(
                value = typed,
                onValueChange = { if (accept(it)) typed = it },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = keyboard, imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { onDone(typed) }),
                modifier = Modifier.width(180.dp).onFocusChanged { if (!it.isFocused && typed != value) onDone(typed) },
            )
        }
        Text(hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
