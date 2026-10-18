package one.aircast.android.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.map.aircast

@Composable
internal fun DeviceSetupScreen(props: DeviceScreenProps) {
    val view = deviceView("setup", ::parseDeviceSetup) ?: return FootNote("Waiting for the device to answer…")
    SetupWizard(view, props.onOpen)
}

@OptIn(ExperimentalLayoutApi::class, ExperimentalFoundationApi::class)
@Composable
private fun SetupWizard(view: SetupView, onOpen: (String) -> Unit) {
    val navigation = LocalAppNavigation.current
    var step by remember { mutableIntStateOf(view.start) }
    var reconfiguring by remember { mutableStateOf(false) }
    var wizardStep by remember { mutableIntStateOf(1) }
    val top = remember { BringIntoViewRequester() }
    LaunchedEffect(step) { top.bringIntoView() }
    val total = view.steps.size
    val current = view.steps[clampStep(step, total) - 1]
    val wizardShown = view.camera.wizard.isNew || reconfiguring
    val nav = setupNav(step, total, cameraHoldsTheWizard(current.key, wizardShown, wizardStep))
    val go = { next: Int -> step = clampStep(next, total) }
    val close = { navigation.settingsOpen = false }
    StepTrail(view.steps, step, Modifier.bringIntoViewRequester(top))
    Column(Modifier.fillMaxWidth().semantics { contentDescription = current.title }) {
        Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 4.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(current.title, style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f).semantics { heading() })
            current.status?.let { DeviceBadgeText(it) }
        }
        DeviceText(current.lead)
        when (current.key) {
            "network" -> InternetStep(view.internet, onOpen)
            "tailscale" -> TailscaleSection(view.remote)
            "camera" -> CameraStep(
                camera = view.camera,
                wizardShown = wizardShown,
                onReconfigure = {
                    wizardStep = 1
                    reconfiguring = true
                },
                onStop = { reconfiguring = false },
                onWizardStep = { wizardStep = it },
                onOpen = onOpen,
            )
            "telemetry" -> TelemetryStep(view.telemetry)
            "done" -> view.checks.map { check ->
                key(check.label) {
                    SetupRow(
                        title = check.label,
                        status = check.status.text,
                        state = if (check.status.level == DeviceLevel.Ready) SetupState.Done else SetupState.Neutral,
                        onClick = { go(check.step) },
                    )
                }
            }
        }
    }
    Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 24.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (nav.back) OutlinedButton(onClick = { go(step - 1) }) { Text("← Back") }
        FlowRow(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (nav.later) TextButton(onClick = close) { Text("Finish setup later") }
            if (nav.next == SetupNext.Next) Button(onClick = { go(step + 1) }) { Text("Next →") }
            if (nav.next == SetupNext.Skip) OutlinedButton(onClick = { go(step + 1) }) { Text("Skip this step →") }
            if (nav.finish) ActionButton("Go to the fly view", DeviceRequest(DeviceMethod.POST, "/api/setup/complete"), onDone = close)
        }
    }
}

@Composable
private fun StepTrail(steps: List<DeviceSetupStep>, step: Int, modifier: Modifier) {
    Row(
        modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 16.dp).semantics { contentDescription = "Setup steps" },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        steps.mapIndexed { index, entry ->
            val here = index + 1 == step
            val done = !here && entry.status != null
            val (fill, edge) = when {
                here -> MaterialTheme.colorScheme.primary to MaterialTheme.colorScheme.primary
                done -> MaterialTheme.aircast.success to Color.Transparent
                else -> Color.Transparent to MaterialTheme.colorScheme.outlineVariant
            }
            key(entry.key) {
                if (index > 0) Box(Modifier.width(8.dp).height(1.dp).background(MaterialTheme.colorScheme.outlineVariant))
                Box(Modifier.size(24.dp).background(fill, CircleShape).border(1.dp, edge, CircleShape).semantics { selected = here }, contentAlignment = Alignment.Center) {
                    if (done) {
                        Icon(painterResource(R.drawable.ic_check), "done", tint = MaterialTheme.colorScheme.surface, modifier = Modifier.size(16.dp))
                    } else {
                        Text("${index + 1}", style = MaterialTheme.typography.labelSmall, color = if (here) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                if (here) Text(entry.label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f, fill = false))
            }
        }
    }
}

@Composable
private fun PageRow(page: DevicePage, onOpen: (String) -> Unit) {
    SetupRow(
        title = page.title,
        subtitle = page.summary,
        icon = devicePageIcon(page.icon),
        state = if (page.needs) SetupState.NeedsAttention else SetupState.Neutral,
        onClick = { onOpen(page.href) },
    )
}

@Composable
private fun InternetStep(internet: SetupInternet, onOpen: (String) -> Unit) {
    DeviceNotice(internet.online)
    WifiJoin(internet.reopen, internet.joinOpen)
    PageRow(internet.connection, onOpen)
    DeviceText(internet.hint)
}

@Composable
private fun CameraStep(camera: SetupCamera, wizardShown: Boolean, onReconfigure: () -> Unit, onStop: () -> Unit, onWizardStep: (Int) -> Unit, onOpen: (String) -> Unit) {
    if (camera.problem.isNotEmpty()) DeviceNotice(DeviceBadge(DeviceLevel.Warn, camera.problem))
    if (wizardShown) {
        key(if (camera.wizard.isNew) "" else camera.wizard.name) {
            CameraWizard(camera.wizard, onCancel = if (camera.wizard.isNew) null else onStop, onStep = onWizardStep, onDone = { onStop() })
        }
    } else {
        ButtonRow {
            if (camera.watch.isNotEmpty()) {
                LinkButton(deviceUrl(camera.watch), "▶ Watch live", LinkLook.Filled, external = true)
                FilledTonalButton(onClick = onReconfigure) { Text("Reconfigure") }
            } else {
                Button(onClick = onReconfigure) { Text("Reconfigure") }
            }
        }
        PageRow(camera.cameras, onOpen)
        DeviceText(camera.hint)
    }
}

@Composable
private fun TelemetryStep(telemetry: SetupTelemetry) {
    telemetry.waiting?.let { DeviceNotice(it) }
    EndpointRows(telemetry.endpoints)
    DeviceText(telemetry.hint)
}
