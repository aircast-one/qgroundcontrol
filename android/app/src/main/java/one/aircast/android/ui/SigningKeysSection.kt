package one.aircast.android.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val MAVLINK_GROUP = "mavlinkSettings"
internal const val SIGNING_KEYS_VIEW = "view.signingKeys"
internal const val SIGNING_ADD_PASSPHRASE = "signingKeys.addPassphrase"
internal const val SIGNING_ADD_RAW = "signingKeys.addRaw"
internal const val SIGNING_GENERATE = "signingKeys.generate"
internal const val SIGNING_REMOVE = "signingKeys.remove"
internal const val SIGNING_EXPORT = "signingKeys.export"
internal const val RAW_KEY_HEX_LENGTH = 64
private const val CLIPBOARD_WIPE_MS = 30_000L

internal data class SigningKeyRow(val name: String, val inUse: Boolean)

internal data class SigningKeys(
    val available: Boolean,
    val vehicle: Boolean,
    val activeKey: String,
    val minPassphraseLength: Int,
    val keys: List<SigningKeyRow>,
)

internal fun signingKeys(view: JSONObject?): SigningKeys? = view?.takeIf { it.optBoolean("available") }?.let {
    val rows = it.optJSONArray("keys")
    SigningKeys(
        available = true,
        vehicle = it.optBoolean("vehicle"),
        activeKey = it.optText("activeKey"),
        minPassphraseLength = it.optInt("minPassphraseLength", 8),
        keys = (0 until (rows?.length() ?: 0)).mapNotNull { at -> rows!!.optJSONObject(at)?.let { row -> SigningKeyRow(row.optText("name"), row.optBoolean("inUse")) } },
    )
}

internal fun isHexKey(text: String): Boolean = text.length == RAW_KEY_HEX_LENGTH && text.all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }

internal fun canAddKey(name: String, raw: Boolean, secret: String, minPassphrase: Int): Boolean =
    name.isNotEmpty() && if (raw) isHexKey(secret) else secret.length >= minPassphrase

@Composable
internal fun SigningKeysSection() {
    var revision by remember { mutableIntStateOf(0) }
    var read by remember { mutableStateOf<SigningKeys?>(null) }
    var adding by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf<String?>(null) }
    var exported by remember { mutableStateOf<String?>(null) }
    var wipeAfterExport by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    LaunchedEffect(revision) {
        read = withContext(Dispatchers.Default) { signingKeys(Qgc.get(SIGNING_KEYS_VIEW)) }
    }
    LaunchedEffect(wipeAfterExport) {
        if (wipeAfterExport > 0) {
            delay(CLIPBOARD_WIPE_MS)
            (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("", ""))
        }
    }
    val keys = read ?: return

    SectionHeader("Signing Keys")
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        if (keys.vehicle) {
            Row(Modifier.fillMaxWidth()) {
                Text("Active Key", modifier = Modifier.weight(1f))
                Text(keys.activeKey)
            }
        }
        keys.keys.forEach { key ->
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(key.name, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                if (!key.inUse) {
                    TextButton(onClick = {
                        scope.launch {
                            val hex = withContext(Dispatchers.Default) { Qgc.invokeResult(SIGNING_EXPORT, key.name) as? String }
                            if (!hex.isNullOrEmpty()) {
                                (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText(key.name, hex))
                                wipeAfterExport++
                                exported = key.name
                            }
                        }
                    }) { Text("Export") }
                    TextButton(onClick = { confirmDelete = key.name }) { Text("Delete") }
                }
            }
        }
        if (keys.keys.isEmpty()) Text("No keys configured", color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedButton(onClick = { adding = true }) { Text("Add Key") }
    }

    if (adding) {
        AddKeyDialog(keys.minPassphraseLength, onDone = { adding = false; revision++ })
    }
    confirmDelete?.let { name ->
        AlertDialog(
            onDismissRequest = { confirmDelete = null },
            title = { Text("Delete Signing Key") },
            text = { Text("Are you sure you want to delete '$name'?\n\nIf a vehicle still has this key configured, you will no longer be able to communicate with it over a signed connection. Raw or generated keys cannot be recovered — Export the hex first if you may need it later.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = null
                    scope.launch {
                        withContext(Dispatchers.Default) { Qgc.invoke(SIGNING_REMOVE, name) }
                        revision++
                    }
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = null }) { Text("Cancel") } },
        )
    }
    exported?.let { name ->
        AlertDialog(
            onDismissRequest = { exported = null },
            title = { Text("Export Key: $name") },
            text = { Text("Key copied to clipboard. Store it securely — it will be cleared from the clipboard in 30 seconds.") },
            confirmButton = { TextButton(onClick = { exported = null }) { Text("OK") } },
        )
    }
}

@Composable
private fun AddKeyDialog(minPassphrase: Int, onDone: () -> Unit) {
    var name by remember { mutableStateOf("") }
    var raw by remember { mutableStateOf(false) }
    var passphrase by remember { mutableStateOf("") }
    var hex by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val secret = if (raw) hex else passphrase

    AlertDialog(
        onDismissRequest = { if (!saving) onDone() },
        title = { Text("Add Signing Key") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Key Name")
                OutlinedTextField(value = name, onValueChange = { name = it }, singleLine = true, placeholder = { Text("Enter a friendly name") })
                Row(verticalAlignment = Alignment.CenterVertically) {
                    RadioButton(selected = !raw, onClick = { raw = false })
                    Text("Passphrase")
                    RadioButton(selected = raw, onClick = { raw = true })
                    Text("Raw Key (hex)")
                }
                if (!raw) {
                    OutlinedTextField(
                        value = passphrase,
                        onValueChange = { passphrase = it },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false),
                        placeholder = { Text("Enter passphrase (min $minPassphrase chars)") },
                    )
                    if (passphrase.isNotEmpty() && passphrase.length < minPassphrase) {
                        Text("Passphrase too short (${passphrase.length}/$minPassphrase)", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                    }
                } else {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        OutlinedTextField(
                            value = hex,
                            onValueChange = { typed -> hex = typed.filter { it.isDigit() || it.lowercaseChar() in 'a'..'f' }.take(RAW_KEY_HEX_LENGTH) },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
                            placeholder = { Text("64 hex characters") },
                            modifier = Modifier.weight(1f),
                        )
                        TextButton(onClick = {
                            scope.launch { hex = withContext(Dispatchers.Default) { Qgc.invokeResult(SIGNING_GENERATE) as? String }.orEmpty() }
                        }) { Text("Generate") }
                    }
                    if (hex.isNotEmpty() && hex.length != RAW_KEY_HEX_LENGTH) {
                        Text("${hex.length}/64 hex characters", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                    }
                }
                error?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
            }
        },
        confirmButton = {
            TextButton(
                enabled = !saving && canAddKey(name, raw, secret, minPassphrase),
                onClick = {
                    saving = true
                    scope.launch {
                        val refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(if (raw) SIGNING_ADD_RAW else SIGNING_ADD_PASSPHRASE, name, secret) }
                        saving = false
                        if (refusal == null) onDone() else error = refusal
                    }
                },
            ) { Text(if (saving) "Adding…" else "OK") }
        },
        dismissButton = { TextButton(enabled = !saving, onClick = onDone) { Text("Cancel") } },
    )
}
