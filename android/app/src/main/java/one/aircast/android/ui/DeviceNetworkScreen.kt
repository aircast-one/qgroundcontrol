package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceNetworkScreen(props: DeviceScreenProps) {
    val view = deviceView("network", ::parseNetwork)
    if (view == null) {
        FootNote("Waiting for the device to answer…")
        return
    }
    PrimaryUplinkSection(view.uplink, props.onOpen)
    view.modem?.let { ModemSection(it) }
    StarlinkSection(view.starlink)
    DeviceSection("Wi-Fi", view.wifi.status) {
        Summary(view.wifi.summary)
        view.wifi.joined?.let { DeviceNotice(it) }
        WifiJoin(view.wifi.reopen)
    }
    InterfacesSection(view.interfaces)
    FootNote("The device routes through the best uplink that reaches the internet and moves off one that stops — this is where you say what it may use.")
    DataSection(view.data)
    FootNote("Mobile data is metered; the budget here is what keeps a flight from spending a month of it — set it before a SIM goes in.")
    TailscaleSection(view.tailscale)
    RelaySection(view.relay)
    FootNote("Reaching this device from somewhere else — it is not needed to fly.")
}

@Composable
private fun Summary(text: String) {
    if (text.isEmpty()) return
    Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 4.dp))
}

@Composable
private fun Leveled(badge: DeviceBadge) {
    Text(badge.text, style = MaterialTheme.typography.bodyMedium, color = deviceLevelColor(badge.level))
}

@Composable
private fun neutralOr(level: Int, neutral: Color): Color = if (level == DeviceLevel.Neutral) neutral else deviceLevelColor(level)

@Composable
private fun Mono(text: String) {
    Text(text, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp))
}

@Composable
private fun ProblemNotice(problem: DeviceProblem) {
    DeviceNotice(DeviceBadge(DeviceLevel.Blocked, problem.text))
    if (problem.detail.isNotEmpty()) DeviceDisclosure("Technical detail") { Mono(problem.detail) }
}

@Composable
private fun PrimaryUplinkSection(uplink: UplinkCard, onOpen: (String) -> Unit) {
    DeviceSection("Primary uplink", uplink.status) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Icon(painterResource(uplinkIcon(uplink.icon)), null, tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(uplink.title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
                Leveled(uplink.verdict)
                if (uplink.detail.isNotEmpty()) Text(uplink.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                listOf(uplink.addr, uplink.meta).filter { it.isNotEmpty() }.takeIf { it.isNotEmpty() }?.let {
                    Text(it.joinToString(" · "), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            if (uplink.rate.isNotEmpty()) {
                Column(horizontalAlignment = Alignment.End) {
                    Text("Sending now", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(uplink.rate, style = MaterialTheme.typography.titleMedium, fontFamily = FontFamily.Monospace)
                }
            }
        }
        if (uplink.warning.isNotEmpty()) {
            DeviceNotice(DeviceBadge(DeviceLevel.Warn, uplink.warning))
            ButtonRow { TextButton(onClick = { onOpen("/streaming") }) { Text("Check the camera →") } }
        }
        uplink.failover.map { row ->
            key(row.label) {
                Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Text(row.label, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Leveled(row.status)
                }
            }
        }
        DeviceText(uplink.switchText)
        DeviceText(uplink.backup)
    }
}

@Composable
private fun ModemSection(modem: ModemCard) {
    DeviceSection("Cellular modem", modem.status) {
        modem.notices.map { notice -> key(notice.text) { DeviceNotice(notice) } }
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
            Text(modem.operator, style = MaterialTheme.typography.bodyLarge)
            Text(modem.detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        modem.signal?.let { DeviceChart(it) }
        modem.facts.map { fact -> key(fact.label) { DeviceFact(fact.label, fact.value) } }
        DeviceDisclosure("Text messages", modem.messages.summary) {
            if (modem.messages.items.isEmpty()) {
                DeviceText("Messages the operator sends to this SIM — package expiry, top-ups, portal codes — show up here.")
            } else {
                modem.messages.items.map { message ->
                    key("${message.time}|${message.text}") {
                        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                                Text(message.from, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                                Text(message.time, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            Text(message.text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
        }
        modem.sim?.let { sim ->
            DeviceDisclosure("SIM & APN", sim.summary) {
                key(sim.apn, sim.metered) {
                    DeviceForm(
                        fields = listOf(
                            DeviceField("apn", "APN", sim.apn, placeholder = sim.placeholder),
                            DeviceField("pin", "SIM PIN", "", hint = "Leave blank to keep the current PIN.", keyboard = KeyboardType.Password),
                            DeviceField("metered", "Data usage", sim.metered, options = sim.options),
                        ),
                        method = DeviceMethod.PUT,
                        path = "/api/network/cellular",
                        submit = "Save",
                        success = "Cellular settings applied.",
                        note = "Re-dials the modem on save.",
                        confirm = ActionConfirm(
                            "Re-dial the modem?",
                            "Saving re-dials the modem — the cellular link drops briefly. If you're connected over cellular you may lose access until it reconnects. Continue?",
                            "Save",
                        ),
                    )
                }
            }
        }
    }
}

@Composable
private fun StarlinkSection(starlink: StarlinkCard) {
    DeviceSection("Starlink", starlink.status) {
        Summary(starlink.summary)
        starlink.demo?.takeIf { it.first }?.let { DemoDish(it) }
        starlink.notices.map { notice -> key(notice.text) { DeviceNotice(notice) } }
        if (starlink.pollError.isNotEmpty()) Mono(starlink.pollError)
        starlink.facts.map { fact -> key(fact.label) { DeviceFact(fact.label, fact.value) } }
        if (starlink.firmware.isNotEmpty()) DeviceFact("Firmware", starlink.firmware)
        DeviceDisclosure("Dish address", initiallyOpen = starlink.address.open) {
            DeviceText("The network address of the Starlink dish. Leave the default unless the dish is reached at a different address on this device's network. Port 9200 is assumed if you leave it off.")
            if (starlink.address.defaultFirst) UseDefaultAddress(starlink.address.fallback)
            key(starlink.address.value) {
                DeviceForm(
                    fields = listOf(DeviceField("addr", "Dish address", starlink.address.value, placeholder = "192.168.100.1:9200", keyboard = KeyboardType.Uri)),
                    method = DeviceMethod.PUT,
                    path = "/api/starlink/config",
                    submit = "Save",
                    success = "Dish address saved.",
                )
            }
            if (!starlink.address.defaultFirst) UseDefaultAddress(starlink.address.fallback)
        }
        starlink.demo?.takeUnless { it.first }?.let { DemoDish(it) }
    }
}

@Composable
private fun UseDefaultAddress(fallback: String) {
    if (fallback.isEmpty()) return
    ButtonRow {
        ActionButton("Use default ($fallback)", DeviceRequest(DeviceMethod.PUT, "/api/starlink/config", JSONObject().put("addr", fallback)), success = "Dish address back to the default.")
    }
}

private fun demoCondition(condition: String): DeviceRequest = DeviceRequest(DeviceMethod.POST, "/api/starlink/demo", JSONObject().put("condition", condition))

@Composable
private fun DemoDish(demo: StarlinkDemo) {
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    DeviceSwitchRow(
        title = "Demo dish",
        detail = demo.note,
        checked = demo.on,
        on = demoCondition("healthy"),
        off = demoCondition("off"),
        success = "Demo dish on — simulated, not a real dish.",
        offSuccess = "Demo dish off.",
    )
    if (demo.on) {
        Box(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
            DeviceSelect("Condition", demo.condition, demo.conditions, { condition -> scope.launch { action.run(demoCondition(condition), "Demo dish condition changed.") } }, enabled = !action.busy)
        }
        RefusalText(action.refusal)
    }
}

private fun ipv4Fields(methods: List<DeviceOption>, ipv4: IPv4Settings): List<DeviceField> = listOf(
    DeviceField("method", "IPv4 method", ipv4.method.ifEmpty { methods.firstOrNull()?.value ?: "auto" }, options = methods),
    DeviceField("metric", "Route metric (priority)", ipv4.metric, placeholder = ipv4.metricPlaceholder, keyboard = KeyboardType.Number),
    DeviceField("address", "Address (CIDR)", ipv4.address, placeholder = "192.168.8.2/24", keyboard = KeyboardType.Uri, shownWhen = { manualAddressing(it["method"].orEmpty()) }),
    DeviceField("gateway", "Gateway", ipv4.gateway, placeholder = "optional", keyboard = KeyboardType.Uri, shownWhen = { manualAddressing(it["method"].orEmpty()) }),
    DeviceField("dns", "DNS", ipv4.dns, placeholder = "8.8.8.8, 1.1.1.1", keyboard = KeyboardType.Uri),
)

private fun newConnectionField(name: String, label: String, placeholder: String = "", hint: String = "", keyboard: KeyboardType = KeyboardType.Text): DeviceField =
    DeviceField(name, label, "", placeholder = placeholder, hint = hint, keyboard = keyboard, shownWhen = { connectionFieldShown(it["type"].orEmpty(), name) })

@Composable
private fun RenameForm(uuid: String, name: String) {
    key(name) {
        DeviceForm(
            fields = listOf(DeviceField("name", "Connection name", name, placeholder = "e.g. Wired connection 2")),
            method = DeviceMethod.PUT,
            path = "/api/network/connections/$uuid/name",
            submit = "Rename",
            success = "Connection renamed.",
        )
    }
}

@Composable
private fun InterfacesSection(interfaces: InterfacesCard) {
    val listed = interfaces.links.size + interfaces.saved.size + interfaces.other.size > 0
    DeviceSection("Interfaces & priority", interfaces.status) {
        Summary(interfaces.summary)
        interfaces.problem?.let { ProblemNotice(it) }
        if (interfaces.problem == null && !listed) DeviceNotice(DeviceBadge(DeviceLevel.Neutral, "No connections."))
        interfaces.links.map { link -> key(link.editor?.uuid?.ifEmpty { null } ?: link.name) { NetLinkRow(link, interfaces.methods) } }
        interfaces.saved.map { profile -> key(profile.uuid) { SavedRow(profile, interfaces.methods) } }
        if (interfaces.problem == null && interfaces.links.size + interfaces.saved.size > 0) {
            DeviceText("The primary uplink carries internet traffic; a backup takes over if it drops. Open a connection to edit it — changes to a running connection apply immediately; inactive ones are saved profiles that aren't running.")
        }
        if (interfaces.other.isNotEmpty()) {
            Text("Other interfaces", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 12.dp))
            interfaces.other.map { usage ->
                key(usage.label) {
                    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
                        Text(usage.label, style = MaterialTheme.typography.bodyLarge)
                        Text("${usage.month} · all-time ${usage.total}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
        DeviceDisclosure("Add a connection", "Create a new NetworkManager profile — ethernet, Wi-Fi or cellular.") {
            DeviceForm(
                fields = listOf(
                    DeviceField("type", "Type", interfaces.add.types.firstOrNull()?.value ?: "ethernet", options = interfaces.add.types),
                    DeviceField("name", "Name", "", placeholder = "camera-net"),
                    DeviceField("interface", "Interface", "", options = listOf(DeviceOption("", "Any interface")) + interfaces.add.ports),
                    DeviceField("autoconnect", "Autoconnect", interfaces.add.autoconnect.firstOrNull()?.value ?: "false", options = interfaces.add.autoconnect),
                    newConnectionField("ssid", "SSID", placeholder = "network name"),
                    newConnectionField("psk", "Passphrase", hint = "Leave blank for an open network.", keyboard = KeyboardType.Password),
                    newConnectionField("apn", "APN", placeholder = "internet"),
                    newConnectionField("pin", "SIM PIN", placeholder = "optional", keyboard = KeyboardType.Password),
                ) + ipv4Fields(interfaces.methods, IPv4Settings("", "", "automatic", "", "", "")),
                method = DeviceMethod.POST,
                path = "/api/network/connections",
                submit = "Create",
                success = "Connection created.",
                note = "Created inactive unless autoconnect is on — it won't disturb the current uplink.",
            )
        }
    }
}

private fun linkSummary(link: NetLink): String =
    listOf(link.kind, link.role?.text ?: if (link.local) "local" else "", link.where, link.usage).filter { it.isNotEmpty() }.joinToString(" · ")

@Composable
private fun NetLinkRow(link: NetLink, methods: List<DeviceOption>) {
    val editor = link.editor
    if (editor == null) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
            Text(link.name, style = MaterialTheme.typography.bodyLarge)
            Text(linkSummary(link), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        return
    }
    DeviceDisclosure(link.name, linkSummary(link)) {
        key(editor.ipv4) {
            DeviceForm(
                fields = ipv4Fields(methods, editor.ipv4),
                method = DeviceMethod.PUT,
                path = "/api/network/interfaces/${editor.iface}",
                submit = "Save",
                success = "${editor.iface} updated.",
                note = "Applies immediately.",
                confirm = editor.confirm.takeIf { it.isNotEmpty() }?.let { ActionConfirm("Reconfigure ${editor.iface}?", it, "Continue") },
            )
        }
        if (editor.uuid.isNotEmpty()) {
            RenameForm(editor.uuid, link.name)
            DeviceText("Bring this connection down, or remove the saved profile.")
            ButtonRow {
                ActionButton(
                    "Deactivate",
                    DeviceRequest(DeviceMethod.POST, "/api/network/connections/${editor.uuid}/deactivate"),
                    success = "${link.name} deactivated.",
                    confirm = ActionConfirm("Deactivate ${link.name}?", editor.deactivateConfirm, "Deactivate"),
                )
                ActionButton(
                    "Delete",
                    DeviceRequest(DeviceMethod.DELETE, "/api/network/connections/${editor.uuid}"),
                    success = "${link.name} deleted.",
                    confirm = ActionConfirm("Delete ${link.name}?", editor.deleteConfirm, "Delete"),
                    tonal = false,
                )
            }
        }
    }
}

@Composable
private fun SavedRow(profile: SavedProfile, methods: List<DeviceOption>) {
    DeviceDisclosure(profile.name, "${profile.kind} · inactive") {
        RenameForm(profile.uuid, profile.name)
        key(profile.ipv4) {
            DeviceForm(
                fields = ipv4Fields(methods, profile.ipv4),
                method = DeviceMethod.PUT,
                path = "/api/network/connections/${profile.uuid}/ipv4",
                submit = "Save",
                success = "${profile.name} updated.",
                note = "Saved to the profile (not activated).",
            )
        }
        ButtonRow {
            ActionButton(
                "Activate",
                DeviceRequest(DeviceMethod.POST, "/api/network/connections/${profile.uuid}/activate"),
                success = "${profile.name} activated.",
                confirm = ActionConfirm("Activate ${profile.name}?", profile.activateConfirm, "Activate"),
            )
            ActionButton(
                "Delete",
                DeviceRequest(DeviceMethod.DELETE, "/api/network/connections/${profile.uuid}"),
                success = "${profile.name} deleted.",
                confirm = ActionConfirm("Delete ${profile.name}?", profile.deleteConfirm, "Delete"),
                tonal = false,
            )
        }
    }
}

@Composable
private fun DataSection(data: DataCard) {
    DeviceSection("Cellular data", data.status) {
        Summary(data.summary)
        data.gauge?.let { Gauge(it) } ?: DeviceNotice(DeviceBadge(DeviceLevel.Neutral, "No usage recorded yet — counters accumulate as data flows."))
        DeviceDisclosure("Data caps & warnings", data.budget.summary) {
            key(data.budget.monthlyCapGB, data.budget.flightCapMB, data.budget.warnPercent) {
                DeviceForm(
                    fields = listOf(
                        DeviceField("monthlyCapGB", "Monthly plan (GB)", data.budget.monthlyCapGB, placeholder = "e.g. 10 — blank for none", keyboard = KeyboardType.Decimal),
                        DeviceField("flightCapMB", "Per-flight budget (MB)", data.budget.flightCapMB, placeholder = "e.g. 800 — blank for none", keyboard = KeyboardType.Decimal),
                        DeviceField("warnPercent", "Warn at (%)", data.budget.warnPercent, placeholder = "80", keyboard = KeyboardType.Number),
                    ),
                    method = DeviceMethod.PUT,
                    path = "/api/usage/budget",
                    submit = "Save",
                    success = "Data caps saved.",
                    note = "Warnings alert the ground station; caps never cut the link.",
                )
            }
        }
    }
}

@Composable
private fun Gauge(gauge: DataGauge) {
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Cellular · LTE", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            gauge.badges.map { badge -> key(badge.text) { DeviceBadgeText(badge) } }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(gauge.hero, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.alignByBaseline())
            Text(gauge.heroCaption, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.alignByBaseline())
        }
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            LinearProgressIndicator(progress = { gauge.uplinkPercent / 100f }, modifier = Modifier.fillMaxWidth().height(8.dp))
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(gauge.uplink, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(gauge.downlink, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        gauge.meters.map { meter -> key(meter.label) { Meter(meter) } }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            gauge.stats.map { stat ->
                key(stat.label) {
                    Column(Modifier.weight(1f)) {
                        Text(stat.label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(stat.value, style = MaterialTheme.typography.titleSmall, fontFamily = FontFamily.Monospace)
                        if (stat.note.isNotEmpty()) Text(stat.note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }
    }
}

@Composable
private fun Meter(meter: BudgetMeter) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(meter.label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
            Text(meter.used, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, color = neutralOr(meter.level, MaterialTheme.colorScheme.onSurface))
        }
        LinearProgressIndicator(progress = { meter.percent / 100f }, modifier = Modifier.fillMaxWidth().height(8.dp), color = neutralOr(meter.level, MaterialTheme.colorScheme.primary))
        Text(meter.remaining, style = MaterialTheme.typography.bodySmall, color = deviceLevelColor(meter.level))
        if (meter.note.isNotEmpty()) Text(meter.note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
internal fun TailscaleSection(tailscale: TailscaleCard) {
    DeviceSection("Tailscale", tailscale.status) {
        Summary(tailscale.summary)
        tailscale.problem?.let { ProblemNotice(it) }
        tailscale.banner?.let { banner ->
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(horizontal = 16.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Column(Modifier.weight(1f)) {
                    Text(banner.title, style = MaterialTheme.typography.titleSmall)
                    Text(banner.subtitle, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                DeviceBadgeText(banner.status)
            }
        }
        tailscale.facts.map { fact -> key(fact.label) { DeviceFact(fact.label, fact.value) } }
        if (tailscale.addresses.isNotEmpty()) DeviceFact("This device's addresses", tailscale.addresses.joinToString(", "))
        tailscale.signIn?.let { signIn ->
            Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                LinkButton(signIn.href, signIn.label, external = true)
                Text(signIn.href, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (tailscale.connect.isNotEmpty() || tailscale.cancel || tailscale.logOut.isNotEmpty()) {
            ButtonRow {
                if (tailscale.connect.isNotEmpty()) ActionButton(tailscale.connect, DeviceRequest(DeviceMethod.POST, "/api/tailscale/connection"))
                if (tailscale.cancel) ActionButton("Cancel sign-in", DeviceRequest(DeviceMethod.POST, "/api/tailscale/connection/cancel"), success = "Sign-in cancelled.", tonal = false)
                if (tailscale.logOut.isNotEmpty()) {
                    ActionButton(
                        "Log out",
                        DeviceRequest(DeviceMethod.DELETE, "/api/tailscale/connection"),
                        success = "Logged out of Tailscale.",
                        confirm = ActionConfirm("Log out of the tailnet?", tailscale.logOut, "Log out"),
                    )
                }
            }
        }
        DeviceText(tailscale.hint)
        tailscale.control?.let { ControlServer(it) }
    }
}

@Composable
private fun ControlServer(control: TailscaleControl) {
    DeviceDisclosure(control.title, initiallyOpen = control.open) {
        key(control.server, control.authKeySet) {
            DeviceForm(
                fields = listOf(
                    DeviceField("controlServer", "Control server URL", control.server, placeholder = "https://headscale.example.com", keyboard = KeyboardType.Uri),
                    DeviceField("authKey", "Auth key — device joins automatically, no sign-in", "", placeholder = control.authKeyPlaceholder, keyboard = KeyboardType.Password),
                ),
                method = DeviceMethod.PUT,
                path = "/api/tailscale/control",
                submit = "Save",
                success = "Control server saved.",
            )
        }
        if (control.authKeySet) {
            ButtonRow {
                ActionButton(
                    "Remove auth key",
                    DeviceRequest(DeviceMethod.DELETE, "/api/tailscale/control/authkey"),
                    success = "Auth key removed.",
                    confirm = ActionConfirm("Remove the auth key?", "Remove the saved auth key? Switching will then need an interactive sign-in over the LAN.", "Remove"),
                    tonal = false,
                )
            }
        }
        if (control.switchError.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Blocked, "Last switch failed (rolled back): ${control.switchError}"))
        DeviceText(control.pending)
        if (control.switchTo != null || control.logOutToSwitch != null) {
            ButtonRow {
                control.switchTo?.let { switchTo ->
                    ActionButton(
                        switchTo.label,
                        DeviceRequest(DeviceMethod.POST, "/api/tailscale/switch"),
                        success = "Switching… if it fails it rolls back automatically.",
                        confirm = ActionConfirm("${switchTo.label}?", switchTo.confirm, "Switch"),
                    )
                }
                control.logOutToSwitch?.let { logOut ->
                    ActionButton(
                        logOut.label,
                        DeviceRequest(DeviceMethod.DELETE, "/api/tailscale/connection"),
                        success = "Logged out — now connect to finish switching.",
                        confirm = ActionConfirm("Log out to switch?", logOut.confirm, "Log out"),
                    )
                }
            }
        }
        DeviceText(control.hint)
    }
}

private fun relayMode(relay: Boolean): DeviceRequest = DeviceRequest(DeviceMethod.PUT, "/api/stream/webrtc-mode", JSONObject().put("relay", relay.toString()))

@Composable
private fun RelaySection(relay: RelayCard) {
    DeviceSection("Relay server (TURN)", relay.status) {
        Summary(relay.summary)
        DeviceSwitchRow(
            title = "Use my relay instead of Tailscale",
            checked = relay.configured && relay.on,
            on = relayMode(true),
            off = relayMode(false),
            detail = if (relay.configured) "" else relay.reason,
            success = "Remote video now goes through your relay.",
            offSuccess = "Remote video goes over Tailscale again.",
            enabled = relay.configured,
        )
        DeviceNotice(relay.notice)
        key(relay.servers) { IceEditor(relay.servers) }
        DeviceDisclosure("Technical details") {
            DeviceText(
                "Off (default) advertises the tailscale 100.x and LAN host candidates. On stops advertising the tailscale host candidate and routes WebRTC media through the configured STUN/TURN ICE servers over UDP, so symmetric-CGNAT sessions on cellular relay over UDP instead of falling back to Tailscale DERP (TCP). Same-LAN viewers still use the direct LAN candidate.",
            )
        }
    }
}

@Composable
private fun IceEditor(servers: List<IceServer>) {
    var rows by remember { mutableStateOf(iceRows(servers)) }
    val action = rememberDeviceAction()
    val scope = rememberCoroutineScope()
    val change = { index: Int, next: IceServer -> rows = rows.mapIndexed { at, row -> if (at == index) next else row } }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Relay (TURN / STUN) servers", style = MaterialTheme.typography.bodyLarge)
        rows.mapIndexed { index, row ->
            key(index) {
                Column(
                    Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(12.dp)).padding(12.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = row.url,
                            onValueChange = { change(index, row.copy(url = it)) },
                            label = { Text("Server URL") },
                            placeholder = { Text("turn:turn.example.com:443?transport=udp") },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
                            modifier = Modifier.weight(1f),
                        )
                        IconButton(onClick = { rows = rows.filterIndexed { at, _ -> at != index } }) { Icon(painterResource(R.drawable.ic_close), "Remove server") }
                    }
                    OutlinedTextField(
                        value = row.username,
                        onValueChange = { change(index, row.copy(username = it)) },
                        label = { Text("Username") },
                        placeholder = { Text("username") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    SecretField("Password", row.password, { change(index, row.copy(password = it)) }, placeholder = "password")
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = { rows = rows + IceServer() }) { Text("+ Add server") }
            TextButton(onClick = { rows = rows + IceServer(url = GOOGLE_STUN) }) { Text("+ Add Google STUN") }
        }
        action.refusal?.let { Text(it.message, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        FilledTonalButton(
            onClick = { scope.launch { iceServerLines(rows)?.let { servers -> action.run(DeviceRequest(DeviceMethod.PUT, "/api/stream/webrtc-ice-servers", JSONObject().put("servers", servers)), "Servers saved.") } } },
            enabled = !action.busy,
        ) { Text("Save servers") }
        Text(
            "A TURN server relays video when a direct connection isn't possible — use one on UDP, ideally port 443. STUN-only lines need no username or password.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
