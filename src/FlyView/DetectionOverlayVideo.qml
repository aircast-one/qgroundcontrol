import QtQuick

import QGroundControl

Item {
    id: root

    property string videoUrl: {
        const vm = QGroundControl.videoManager
        return vm.activeVideoSource >= 0 && vm.activeVideoSource < vm.cameraUrls.length ? vm.cameraUrls[vm.activeVideoSource] : ""
    }
    property int    pollInterval: 100
    property int    staleAfterMs: 1000

    readonly property var    _parsed:  _parse(videoUrl)
    readonly property string _apiBase: _parsed.host ? "http://" + _parsed.host : ""
    readonly property string _camera:  _parsed.camera
    property var _boxes: []

    visible: _apiBase !== "" && _camera !== ""

    function _parse(u) {
        var m = /^[a-z][a-z0-9+.-]*:\/\/(?:[^@\/]*@)?([^\/:?#]+)/i.exec(u || "")
        if (!m) return { host: "", camera: "" }
        var host = m[1]
        var afterHost = (u.split(host)[1] || "")
        var path = afterHost.replace(/^:\d+/, "").replace(/[?#].*$/, "").replace(/^\/+|\/+$/g, "")
        if (path === "") return { host: host, camera: "" }
        var seg = path.split("/")
        var last = seg[seg.length - 1]
        if ((last === "whep" || last === "whip") && seg.length > 1) {
            last = seg[seg.length - 2]
        }
        return { host: host, camera: last }
    }

    function _poll() {
        if (_apiBase === "" || _camera === "") return
        var xhr = new XMLHttpRequest()
        xhr.open("GET", _apiBase + "/api/streams/" + encodeURIComponent(_camera) + "/detections")
        xhr.onreadystatechange = function() {
            if (xhr.readyState !== XMLHttpRequest.DONE) return
            if (xhr.status !== 200) { root._boxes = []; canvas.requestPaint(); return }
            try {
                var r = JSON.parse(xhr.responseText)
                root._boxes = (r.ageMs >= 0 && r.ageMs <= root.staleAfterMs && r.boxes) ? r.boxes : []
            } catch (e) {
                root._boxes = []
            }
            canvas.requestPaint()
        }
        xhr.send()
    }

    Timer {
        interval: root.pollInterval
        running:  root.visible
        repeat:   true
        onTriggered: root._poll()
    }

    Canvas {
        id: canvas
        anchors.fill: parent
        renderStrategy: Canvas.Cooperative

        onPaint: {
            var ctx = getContext("2d")
            ctx.clearRect(0, 0, width, height)
            if (width <= 0 || height <= 0) return

            var lineW = Math.max(2, Math.round(height * 0.004))
            var fontPx = Math.max(11, Math.round(height * 0.025))
            ctx.lineWidth = lineW
            ctx.font = "bold " + fontPx + "px sans-serif"
            ctx.textBaseline = "alphabetic"

            for (var i = 0; i < root._boxes.length; i++) {
                var b = root._boxes[i]
                var x = b.x * width
                var y = b.y * height
                var w = b.w * width
                var h = b.h * height

                ctx.strokeStyle = "#00e676"
                ctx.strokeRect(x, y, w, h)

                var label = String(b.label) + " " + Math.round(b.conf * 100) + "%"
                var tw = ctx.measureText(label).width
                var th = fontPx + 4
                var ly = y - th < 0 ? y : y - th
                ctx.fillStyle = "#00e676"
                ctx.fillRect(x, ly, tw + 8, th)
                ctx.fillStyle = "#000000"
                ctx.fillText(label, x + 4, ly + fontPx)
            }
        }
    }
}
