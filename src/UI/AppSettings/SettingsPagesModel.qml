/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQml.Models

import QGroundControl
import QGroundControl.ScreenTools

ListModel {
    function pages() {
        return Array.from({ length: count }, (unused, index) => get(index))
    }

    function matches(page, filter) {
        const needle = filter.trim().toLowerCase()
        return page.pageVisible() &&
               (needle === "" ||
                page.name.toLowerCase().includes(needle) ||
                page.summary.toLowerCase().includes(needle) ||
                page.keywords.toLowerCase().includes(needle))
    }

    function matchCount(filter) {
        return pages().filter(page => matches(page, filter)).length
    }

    ListElement {
        name: qsTr("General")
        summary: qsTr("Appearance, files, units")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("language, locale, units, metric, imperial, feet, meters, metres, altitude, distance, speed, temperature, area, colour, color, scheme, theme, dark, light, glass, frost, blur, ui, scaling, font, size, brand, image, load, save, path, files, sd, card, audio, mute, sound, stream, gcs, position, follow, me, reset, all, settings")
        section: qsTr("Application")
        url: "qrc:/qml/QGroundControl/AppSettings/GeneralSettings.qml"
        iconUrl: "qrc:/res/QGCLogoWhite.svg"
        tileColor: "#8e8e93"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Fly View")
        summary: qsTr("Guided commands, instruments, camera controls")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("preflight, checklist, multi-vehicle, panel, map, centered, telemetry, log, replay, status, bar, camera, controls, digicam, photo, video, recording, return, to, home, guided, commands, minimum, maximum, altitude, go, to, location, loiter, radius, mavlink, actions, joystick, virtual, joystick, throttle, left-handed, instrument, panel, compass, heading, nose-up, gimbal, tilt, pan, zoom, light, record, rc, channel, custom, controls, slider, toggle, switch, momentary, 3d, view, osm, building, height")
        section: qsTr("Views")
        url: "qrc:/qml/QGroundControl/AppSettings/FlyViewSettings.qml"
        iconUrl: "qrc:/qmlimages/PaperPlane.svg"
        tileColor: "#0a84ff"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Plan View")
        summary: qsTr("Mission defaults and pattern options")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("default, mission, altitude, vtol, transition, distance, condition, gate, pattern, generation, takeoff, item, landing, sequences, survey, corridor, structure, scan")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/PlanViewSettings.qml"
        iconUrl: "qrc:/qmlimages/Plan.svg"
        tileColor: "#30d158"
        newSection: false
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Video")
        summary: qsTr("Cameras, stream, local storage")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("camera, name, source, url, stream, rtsp, udp, tcp, connection, timeout, aspect, ratio, stop, recording, disarmed, low, latency, decode, priority, local, storage, file, format, delete, old, recordings, storage, limit")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/VideoSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/camera.svg"
        tileColor: "#ff375f"
        newSection: false
        pageVisible: function() { return QGroundControl.settingsManager.videoSettings.visible }
    }

    ListElement {
        name: qsTr("Telemetry")
        summary: qsTr("Logging, forwarding, stream rates")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("ground, station, mavlink, system, id, heartbeat, mavlink, 2, signing, key, forwarding, host, name, logging, flight, log, csv, stream, rates, ardupilot")
        section: qsTr("Vehicle & Links")
        url: "qrc:/qml/QGroundControl/AppSettings/TelemetrySettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/drone.svg"
        tileColor: "#5e5ce6"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Packet Radio")
        summary: qsTr("Link quality, radio, adaptive link")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("link, quality, signal, antenna, noise, margin, link, score, packets, lost, video, packets, radio, wifi, wi-fi, adapter, adaptive, link, wfb")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/PacketRadioSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/radio.svg"
        tileColor: "#ff9f0a"
        newSection: false
        pageVisible: function() { return QGroundControl.settingsManager.packetRadioSettings.visible }
    }

    ListElement {
        name: qsTr("ADSB Server")
        summary: qsTr("ADS-B traffic feed")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("adsb, ads-b, traffic, feed, transponder, aircraft, host, port")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/ADSBServerSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/airplane.svg"
        tileColor: "#64d2ff"
        newSection: false
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Comm Links")
        summary: qsTr("Auto connect, NMEA GPS, links")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("auto, connect, nmea, gps, device, baudrate, udp, port, serial, tcp, bluetooth, link, type, options, connection")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/LinkSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/usb.svg"
        tileColor: "#bf5af2"
        newSection: false
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Maps")
        summary: qsTr("Offline maps, tokens, tile cache")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("map, provider, type, elevation, provider, offline, maps, import, export, tiles, tokens, mapbox, esri, vworld, account, map, style, custom, url, server, tile, cache")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/MapSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/globe.svg"
        tileColor: "#30d158"
        newSection: false
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Remote ID")
        summary: qsTr("Basic ID, operator ID, location")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("remote, id, arm, status, gcs, gps, basic, id, operator, id, self, id, broadcast, message, ground, station, location, nmea, baudrate, udp, port, eu, vehicle, info")
        section: qsTr("Compliance")
        url: "qrc:/qml/QGroundControl/AppSettings/RemoteIDSettings.qml"
        iconUrl: "qrc:/qmlimages/RidIconManNoID.svg"
        tileColor: "#0a84ff"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("PX4 Log Transfer")
        summary: qsTr("MAVLink logging, upload, saved files")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("px4, mavlink, logging, log, upload, email, address, description, upload, url, video, url, wind, speed, flight, rating, logs, saved, files")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/PX4LogTransferSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/inbox-download.svg"
        tileColor: "#ac8e68"
        newSection: false
        pageVisible: function() { 
            var activeVehicle = QGroundControl.multiVehicleManager.activeVehicle
            return QGroundControl.corePlugin.options.showPX4LogTransferOptions && 
                        QGroundControl.px4ProFirmwareSupported && 
                        (activeVehicle ? activeVehicle.px4Firmware : true)
        }
    }

    ListElement {
        name: qsTr("Help")
        summary: qsTr("About and support")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("about, version, support, documentation, help")
        section: qsTr("Support")
        url: "qrc:/qml/QGroundControl/AppSettings/HelpSettings.qml"
        iconUrl: "qrc:/InstrumentValueIcons/question.svg"
        tileColor: "#0a84ff"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Console")
        summary: qsTr("Application messages")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("application, messages, console, log, output")
        section: qsTr("Diagnostics")
        url: "qrc:/qml/QGroundControl/Controls/AppMessages.qml"
        iconUrl: "qrc:/InstrumentValueIcons/conversation.svg"
        tileColor: "#8e8e93"
        newSection: true
        pageVisible: function() { return true }
    }

    ListElement {
        name: qsTr("Mock Link")
        summary: qsTr("Simulated vehicle link")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("mock, link, simulated, vehicle, test")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/MockLink.qml"
        iconUrl: "qrc:/InstrumentValueIcons/drone.svg"
        tileColor: "#ffd60a"
        newSection: false
        pageVisible: function() { return ScreenTools.isDebug }
    }

    ListElement {
        name: qsTr("Debug")
        summary: qsTr("Debug tools")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("debug, tools, developer")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/DebugWindow.qml"
        iconUrl: "qrc:/InstrumentValueIcons/bug.svg"
        tileColor: "#ff453a"
        newSection: false
        pageVisible: function() { return ScreenTools.isDebug }
    }

    ListElement {
        name: qsTr("Palette Test")
        summary: qsTr("Palette preview")
        //: Comma-separated search terms for this settings page. Translate the words a
        //: user would type to look for these settings; keep product names as-is.
        keywords: qsTr("palette, preview, colours, colors, theme, test")
        section: ""
        url: "qrc:/qml/QGroundControl/AppSettings/QmlTest.qml"
        iconUrl: "qrc:/InstrumentValueIcons/photo.svg"
        tileColor: "#ff375f"
        newSection: false
        pageVisible: function() { return ScreenTools.isDebug }
    }
}
