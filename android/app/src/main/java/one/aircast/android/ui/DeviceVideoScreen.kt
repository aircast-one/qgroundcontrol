package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import androidx.compose.ui.text.input.KeyboardType
import one.aircast.android.R
import org.json.JSONObject

@Composable
internal fun DeviceVideoScreen(props: DeviceScreenProps) {
    val view = deviceView("video", ::parseVideo) ?: return FootNote("Waiting for the device to answer…")
    view.radio?.let { RadioSection(it) }
    SharingSection(view.sharing)
    StatusLine("These apply to every camera on this device. Which camera goes where is set on the camera itself.")
    SetupRow(title = view.engineering.title, subtitle = view.engineering.summary, icon = R.drawable.ic_terminal, onClick = { props.onOpen(view.engineering.href) })
}

@Composable
private fun RadioSection(radio: RadioSettings) {
    DeviceSection("Packet radio", radio.status) {
        DeviceText("The channel has to match the ground station exactly — a mismatch looks the same as a dead radio. Saving restarts the radio, so the link drops for a few seconds.")
        key("${radio.channel}|${radio.region}|${radio.txPower}") {
            DeviceForm(
                fields = listOf(
                    DeviceField("channel", "Channel", radio.channel, hint = "1-14 on 2.4 GHz, 32-177 on 5 GHz. 165 is 5825 MHz.", keyboard = KeyboardType.Number),
                    DeviceField("region", "Region", radio.region, hint = "Two-letter country code for the regulatory domain. BO permits the highest transmit power."),
                    DeviceField(
                        "tx_power",
                        "Transmit power",
                        radio.txPower,
                        placeholder = "driver default",
                        hint = "0-30. Leave empty for the driver default — the legal ceiling and the heat the adapter can shed are both airframe-specific.",
                        keyboard = KeyboardType.Number,
                    ),
                ),
                method = DeviceMethod.PUT,
                path = "/api/stream/wfb/radio",
                submit = "Save",
                success = "Radio settings applied.",
                confirm = ActionConfirm("Restart the radio?", "Change the radio and restart it? The link drops for a few seconds — do this on the ground.", "Restart"),
                note = "Restarts the radio.",
            )
        }
        DeviceDisclosure("Technical details") { DeviceText(radio.details) }
    }
}

@Composable
private fun SharingSection(sharing: PublicSharing) {
    DeviceSection("Public viewer (Cloudflare)", sharing.status) {
        when (sharing.stage) {
            SharingStage.Live -> {
                DeviceText("Public sharing is on. Every camera that publishes to Cloudflare shows a public share link — hand it to anyone, no login or VPN, and it survives the backend going down.")
                CopyRow("Bridge Worker", sharing.baseUrl)
                ButtonRow {
                    ActionButton(
                        label = "Turn off",
                        request = DeviceRequest(DeviceMethod.PUT, "/api/stream/cfbridge", JSONObject(SHARING_OFF)),
                        success = "Public sharing turned off.",
                        confirm = ActionConfirm("Turn off public sharing?", "Turn off public sharing? Existing share links stop working.", "Turn off"),
                    )
                }
            }
            SharingStage.Deploy -> {
                StepText(1, "Deploy the bridge to your Cloudflare account", "One click — Cloudflare copies the bridge into your account and creates its storage. Sign in when asked.")
                ButtonRow { LinkButton(sharing.deployUrl, "Deploy to Cloudflare", external = true) }
                StepText(2, "Paste these two keys when Cloudflare asks for secrets")
                CopyRow("SIGN_KEY", sharing.signKey, hint = "Signs share links (generated on this device).")
                CopyRow("DEVICE_TOKEN", sharing.deviceToken, hint = "Lets this device report the live session (generated on this device).")
                StatusLine("Your Cloudflare app secret is sent automatically by this device — you don't need to set it.")
                StepText(3, "Paste the Worker URL Cloudflare gives you")
                DeviceForm(
                    fields = listOf(DeviceField("baseUrl", "Worker URL", sharing.baseUrl, placeholder = "https://aircast-cf-bridge.<you>.workers.dev", keyboard = KeyboardType.Uri)),
                    method = DeviceMethod.PUT,
                    path = "/api/stream/cfbridge",
                    submit = "Connect",
                    success = "Public sharing is on.",
                    extra = mapOf("signKey" to sharing.signKey, "deviceToken" to sharing.deviceToken),
                )
                ButtonRow {
                    ActionButton(
                        label = "Start over",
                        request = DeviceRequest(DeviceMethod.PUT, "/api/stream/cfbridge", JSONObject(SHARING_OFF)),
                        success = "Setup cleared.",
                        confirm = ActionConfirm("Start over?", "Discard the generated keys and start over?", "Discard"),
                        tonal = false,
                    )
                }
            }
            else -> {
                DeviceText(
                    "Give a camera that publishes to Cloudflare Realtime a public share link — anyone opens it in a browser, no login and no VPN, and it keeps working even if the cloud backend is down. " +
                        "You host the bridge in your own Cloudflare account (free tier is plenty).",
                )
                ButtonRow { ActionButton("Set up public sharing", DeviceRequest(DeviceMethod.POST, "/api/stream/cfbridge/keys"), "Keys generated — follow the deploy steps.") }
            }
        }
    }
}

