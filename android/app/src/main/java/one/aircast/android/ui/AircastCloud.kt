package one.aircast.android.ui

import android.content.Intent
import one.aircast.android.bridge.AccountCommands
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.map.optText
import org.json.JSONObject

internal const val AIRCAST_CLOUD_LINK = "aircastCloud"
internal const val AIRCAST_CLOUD_NAME = "Aircast cloud"
private const val ACCOUNT = "account"
private const val ACCOUNT_POLL_MS = 1000L
private val API_BASE = Regex("""^https?://[^/]+""")

internal data class AccountState(val signedIn: Boolean, val signingIn: Boolean, val status: String, val userCode: String, val verificationUrl: String)

internal fun accountState(view: JSONObject?): AccountState? = view?.takeIf { it.optText("kind") == "object" }?.let {
    AccountState(it.optBoolean("signedIn"), it.optBoolean("signingIn"), it.optText("status"), it.optText("userCode"), it.optText("verificationUrl"))
}

internal fun accountLine(state: AccountState): String = if (state.signedIn) "Signed in" else state.status.ifBlank { "Not signed in" }

internal fun cloudApiBaseValid(apiBase: String): Boolean = API_BASE.containsMatchIn(apiBase.trim())

internal fun cloudDeviceValid(deviceId: String): Boolean = deviceId.isNotBlank()

@Composable
internal fun AircastCloudFields(apiBase: String, deviceId: String, showErrors: Boolean, onApiBase: (String) -> Unit, onDeviceId: (String) -> Unit) {
    var account by remember { mutableStateOf<AccountState?>(null) }
    var opened by remember { mutableStateOf("") }
    val context = LocalContext.current
    LaunchedEffect(Unit) {
        while (true) {
            account = withContext(Dispatchers.Default) { accountState(Qgc.get(ACCOUNT)) }
            delay(ACCOUNT_POLL_MS)
        }
    }
    val pending = account?.takeIf { it.signingIn && it.verificationUrl.isNotBlank() }
    LaunchedEffect(pending?.verificationUrl) {
        val url = pending?.verificationUrl ?: return@LaunchedEffect
        if (url != opened) {
            opened = url
            runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        OutlinedTextField(value = apiBase, onValueChange = onApiBase, label = { Text("Account server") }, placeholder = { Text("https://api.aircast.one") }, singleLine = true, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri))
        if (showErrors && !cloudApiBaseValid(apiBase)) Text("Enter the account server address, starting with https://", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        OutlinedTextField(value = deviceId, onValueChange = onDeviceId, label = { Text("Device") }, singleLine = true)
        if (showErrors && !cloudDeviceValid(deviceId)) Text("Set up from the device to fill this in", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        account?.let { state ->
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("Account  ${accountLine(state)}", modifier = Modifier.weight(1f))
                when {
                    state.signedIn -> OutlinedButton(onClick = { offMainDetached { AccountCommands.signOut() } }) { Text("Sign out") }
                    state.signingIn -> OutlinedButton(onClick = { offMainDetached { AccountCommands.cancelSignIn() } }) { Text("Cancel") }
                    else -> OutlinedButton(onClick = {
                        offMainDetached {
                            AccountCommands.setApiBase(apiBase)
                            AccountCommands.signIn()
                        }
                    }) { Text("Sign in") }
                }
            }
            if (state.signingIn) Text("Your browser opened ${state.verificationUrl} — approve code ${state.userCode} there.", style = MaterialTheme.typography.bodySmall)
        }
    }
}
