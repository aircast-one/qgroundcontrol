package one.aircast.android.ui

import one.aircast.android.R
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

private const val VIEW = """{
  "uplink": {
    "icon": "wifi",
    "title": "Wi-Fi (wlan0)",
    "status": { "level": 0, "text": "carrying traffic" },
    "verdict": { "level": 3, "text": "No video going to the cloud" },
    "addr": "192.168.1.20/24",
    "failover": [{ "label": "LTE (wwan0)", "status": { "level": 3, "text": "standing by · takes over in ~15 s" } }],
    "switch": "Moved just now: from Wi-Fi (wlan0) to LTE (wwan0)"
  },
  "modem": {
    "status": { "level": 0, "text": "connected" },
    "summary": "HARNESS",
    "notices": [],
    "operator": "HARNESS",
    "detail": "lte · home · signal 72%",
    "signal": { "label": "Signal", "values": [80, -1, 72], "max": 100, "value": "72%", "level": 3 },
    "facts": [{ "label": "APN (in use)", "value": "internet" }],
    "messages": { "summary": "1 on the SIM · latest from MAGTICOM", "items": [{ "from": "MAGTICOM", "text": "code 2157", "when": "Sep 25 17:41" }] },
    "sim": { "summary": "APN internet", "apn": "internet", "placeholder": "auto", "metered": "auto", "options": [{ "value": "auto", "label": "Automatic" }] }
  },
  "starlink": { "status": { "level": 3, "text": "no dish" }, "summary": "", "notices": [], "facts": [], "address": { "value": "10.0.0.9:9200", "default": "192.168.100.1:9200", "defaultFirst": true, "open": true } },
  "wifi": { "status": { "level": 3, "text": "not joined" }, "summary": "join a network", "reopen": { "host": "harness.local", "hostname": "harness" } },
  "interfaces": {
    "status": { "level": 3, "text": "" },
    "summary": "1 in use · 1 known",
    "links": [
      {
        "name": "camera (eth0)",
        "kind": "ethernet",
        "where": "eth0 · 192.168.8.2/24",
        "local": true,
        "editor": { "interface": "eth0", "ipv4": { "method": "manual", "metric": "", "metricPlaceholder": "automatic", "address": "192.168.8.2/24", "gateway": "", "dns": "" }, "uuid": "uuid-eth0", "deleteConfirm": "Delete?" }
      }
    ],
    "saved": [{ "uuid": "uuid-spare", "name": "spare-net", "kind": "ethernet", "ipv4": { "method": "auto" }, "activateConfirm": "Activate spare-net?", "deleteConfirm": "Delete spare-net?" }],
    "other": [],
    "methods": [{ "value": "auto", "label": "DHCP (auto)" }],
    "add": { "types": [{ "value": "ethernet", "label": "Ethernet" }], "ports": [], "autoconnect": [] }
  },
  "data": { "status": { "level": 3, "text": "no caps" }, "summary": "0 B this month", "budget": { "summary": "no caps set", "monthlyCapGB": "", "flightCapMB": "", "warnPercent": "0" } },
  "tailscale": {
    "status": { "level": 1, "text": "signing in" },
    "summary": "finish the sign-in",
    "banner": { "title": "Waiting for sign-in", "subtitle": "Step 1 of 2", "status": { "level": 1, "text": "pending" } },
    "facts": [],
    "addresses": [],
    "signIn": { "label": "Open Tailscale login", "href": "https://login.tailscale.com/a/x" },
    "cancel": true
  },
  "relay": { "status": { "level": 3, "text": "off" }, "summary": "", "on": false, "configured": false, "reason": "Add a relay server first", "notice": { "level": 3, "text": "Video keeps working." }, "servers": [] }
}"""

class DeviceNetworkTest {
    @Test
    fun `reads the network view the device serves`() {
        val view = checkNotNull(parseNetwork(JSONObject(VIEW)))
        assertEquals(UplinkBackup("LTE (wwan0)", DeviceBadge(DeviceLevel.Neutral, "standing by · takes over in ~15 s")), view.uplink.failover.first())
        assertTrue(view.uplink.switchText.contains("from Wi-Fi (wlan0) to LTE (wwan0)"))
        assertEquals(listOf(80.0, -1.0, 72.0), view.modem?.signal?.values)
        assertEquals("MAGTICOM", view.modem?.messages?.items?.first()?.from)
        assertEquals("internet", view.modem?.sim?.apn)
        assertEquals(StarlinkAddress("10.0.0.9:9200", "192.168.100.1:9200", defaultFirst = true, open = true), view.starlink.address)
        assertNull(view.starlink.demo)
        assertNull(view.wifi.joined)
        assertEquals(WifiReopen("harness.local", "harness"), view.wifi.reopen)
        assertEquals("eth0", view.interfaces.links.first().editor?.iface)
        assertNull(view.interfaces.links.first().role)
        assertEquals("auto", view.interfaces.saved.first().ipv4.method)
        assertNull(view.data.gauge)
        assertEquals("https://login.tailscale.com/a/x", view.tailscale.signIn?.href)
        assertNull(view.tailscale.control)
        assertEquals("Add a relay server first", view.relay.reason)
        assertNull(parseNetwork(JSONObject(VIEW).apply { remove("modem") })?.modem)
        assertNull(parseNetwork(JSONObject().put("modem", JSONObject(VIEW).getJSONObject("modem"))))
    }

    @Test
    fun `each kind of uplink gets an icon of its own, anything else the network one`() {
        assertEquals(R.drawable.ic_satellite_alt, uplinkIcon("satellite-dish"))
        assertEquals(R.drawable.ic_signal_cellular_alt, uplinkIcon("signal"))
        assertEquals(R.drawable.ic_settings_ethernet, uplinkIcon("ethernet-port"))
        assertEquals(R.drawable.ic_lan, uplinkIcon("network"))
        assertEquals(R.drawable.ic_lan, uplinkIcon(""))
    }

    @Test
    fun `relay servers are sent as the lines the core makes of them, and not at all when it refuses`() {
        assertEquals(GOOGLE_STUN, parseIceServerLines(JSONObject().put("ok", true).put("result", GOOGLE_STUN)))
        assertEquals("", parseIceServerLines(JSONObject().put("ok", true).put("result", "")))
        assertNull(parseIceServerLines(JSONObject().put("ok", false).put("reason", "no")))
        assertNull(parseIceServerLines(null))
    }

    @Test
    fun `an empty server list still offers one row to fill in`() {
        assertEquals(listOf(IceServer("", "", "")), iceRows(emptyList()))
        val saved = listOf(IceServer(GOOGLE_STUN, "", ""))
        assertSame(saved, iceRows(saved))
    }

    @Test
    fun `a new connection shows only the fields its type needs, and manual addressing only when chosen`() {
        assertTrue(connectionFieldShown("wifi", "ssid"))
        assertFalse(connectionFieldShown("wifi", "apn"))
        assertTrue(connectionFieldShown("gsm", "pin"))
        assertFalse(connectionFieldShown("ethernet", "psk"))
        assertTrue(connectionFieldShown("ethernet", "name"))
        assertTrue(manualAddressing("manual"))
        assertFalse(manualAddressing("auto"))
        val fields = listOf(
            DeviceField("method", "IPv4 method", "auto"),
            DeviceField("address", "Address", "", shownWhen = { manualAddressing(it["method"].orEmpty()) }),
        )
        assertEquals(listOf("method"), shownFields(fields, mapOf("method" to "auto")).map { it.name })
        assertEquals(listOf("method", "address"), shownFields(fields, mapOf("method" to "manual")).map { it.name })
    }
}
