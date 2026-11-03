import SwiftUI

struct DeviceVideoScreen: View {
    let props: DeviceScreenProps
    @DeviceView("video") private var reading

    var body: some View {
        if let view = deviceView(reading, parseVideo) {
            if let radio = view.radio { RadioSection(radio: radio) }
            SharingSection(sharing: view.sharing)
            DeviceStatusLine(text: "These apply to every camera on this device. Which camera goes where is set on the camera itself.")
            SetupRow(title: view.engineering.title, onClick: { props.onOpen(view.engineering.href) }, icon: .terminal, subtitle: view.engineering.summary)
        } else {
            FootNote(text: DEVICE_WAITING)
        }
    }
}

private struct RadioSection: View {
    let radio: RadioSettings

    var body: some View {
        DeviceSection(title: "Packet radio", badge: radio.status) {
            DeviceText(text: "The channel has to match the ground station exactly — a mismatch looks the same as a dead radio. Saving restarts the radio, so the link drops for a few seconds.")
            DeviceForm(
                fields: [
                    DeviceField(name: "channel", label: "Channel", value: radio.channel, hint: "1-14 on 2.4 GHz, 32-177 on 5 GHz. 165 is 5825 MHz.", keyboard: .number),
                    DeviceField(name: "region", label: "Region", value: radio.region, hint: "Two-letter country code for the regulatory domain. BO permits the highest transmit power."),
                    DeviceField(
                        name: "tx_power",
                        label: "Transmit power",
                        value: radio.txPower,
                        placeholder: "driver default",
                        hint: "0-30. Leave empty for the driver default — the legal ceiling and the heat the adapter can shed are both airframe-specific.",
                        keyboard: .number
                    ),
                ],
                method: .PUT,
                path: "/api/stream/wfb/radio",
                submit: "Save",
                success: "Radio settings applied.",
                confirm: ActionConfirm(title: "Restart the radio?", text: "Change the radio and restart it? The link drops for a few seconds — do this on the ground.", confirm: "Restart"),
                note: "Restarts the radio."
            )
            .id("\(radio.channel)|\(radio.region)|\(radio.txPower)")
            DeviceDisclosure("Technical details") { DeviceText(text: radio.details) }
        }
    }
}

private struct SharingSection: View {
    let sharing: PublicSharing

    var body: some View {
        DeviceSection(title: "Public viewer (Cloudflare)", badge: sharing.status) {
            switch sharing.stage {
            case SharingStage.Live:
                DeviceText(text: "Public sharing is on. Every camera that publishes to Cloudflare shows a public share link — hand it to anyone, no login or VPN, and it survives the backend going down.")
                CopyRow(label: "Bridge Worker", text: sharing.baseUrl)
                DeviceButtonRow {
                    ActionButton(
                        label: "Turn off",
                        request: DeviceRequest(method: .PUT, path: "/api/stream/cfbridge", body: deviceBody(SHARING_OFF)),
                        success: "Public sharing turned off.",
                        confirm: ActionConfirm(title: "Turn off public sharing?", text: "Turn off public sharing? Existing share links stop working.", confirm: "Turn off")
                    )
                }
            case SharingStage.Deploy:
                StepText(step: 1, title: "Deploy the bridge to your Cloudflare account", text: "One click — Cloudflare copies the bridge into your account and creates its storage. Sign in when asked.")
                DeviceButtonRow { LinkButton(href: sharing.deployUrl, text: "Deploy to Cloudflare", external: true) }
                StepText(step: 2, title: "Paste these two keys when Cloudflare asks for secrets")
                CopyRow(label: "SIGN_KEY", text: sharing.signKey, hint: "Signs share links (generated on this device).")
                CopyRow(label: "DEVICE_TOKEN", text: sharing.deviceToken, hint: "Lets this device report the live session (generated on this device).")
                DeviceStatusLine(text: "Your Cloudflare app secret is sent automatically by this device — you don't need to set it.")
                StepText(step: 3, title: "Paste the Worker URL Cloudflare gives you")
                DeviceForm(
                    fields: [DeviceField(name: "baseUrl", label: "Worker URL", value: sharing.baseUrl, placeholder: "https://aircast-cf-bridge.<you>.workers.dev", keyboard: .uri)],
                    method: .PUT,
                    path: "/api/stream/cfbridge",
                    submit: "Connect",
                    success: "Public sharing is on.",
                    extra: ["signKey": sharing.signKey, "deviceToken": sharing.deviceToken]
                )
                DeviceButtonRow {
                    ActionButton(
                        label: "Start over",
                        request: DeviceRequest(method: .PUT, path: "/api/stream/cfbridge", body: deviceBody(SHARING_OFF)),
                        success: "Setup cleared.",
                        confirm: ActionConfirm(title: "Start over?", text: "Discard the generated keys and start over?", confirm: "Discard"),
                        tonal: false
                    )
                }
            default:
                DeviceText(
                    text: "Give a camera that publishes to Cloudflare Realtime a public share link — anyone opens it in a browser, no login and no VPN, and it keeps working even if the cloud backend is down. " +
                        "You host the bridge in your own Cloudflare account (free tier is plenty)."
                )
                DeviceButtonRow {
                    ActionButton(label: "Set up public sharing", request: DeviceRequest(method: .POST, path: "/api/stream/cfbridge/keys"), success: "Keys generated — follow the deploy steps.")
                }
            }
        }
    }
}
