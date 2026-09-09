import sys
from pathlib import Path

SRC = Path("/Users/pavliha/Code/aircast/qgroundcontrol/src/Bridge/QGCBridgeCore.cc")

PLAIN = """    void _poll()
    {
        if (!g_eventHandler) {
            return;
        }
        _tick++;
        for (const QString &path : std::as_const(_paths)) {
            const auto bound = _bound.constFind(path);
            if (bound != _bound.constEnd()) {
                if (!bound->fact && (_tick % kBoundPropertyRereadTicks) == 0) {
                    _emit(path);
                }
                continue;
            }
            (void) _bind(path);
            _emit(path);
        }
    }"""

PROBED = """    void _poll()
    {
        if (!g_eventHandler) {
            return;
        }
        _tick++;
        QElapsedTimer probeTimer;
        probeTimer.start();
        int unbound = 0;
        int reread = 0;
        for (const QString &path : std::as_const(_paths)) {
            const auto bound = _bound.constFind(path);
            if (bound != _bound.constEnd()) {
                if (!bound->fact && (_tick % kBoundPropertyRereadTicks) == 0) {
                    const qint64 before = probeTimer.nsecsElapsed();
                    _emit(path);
                    _probeCost[path] += probeTimer.nsecsElapsed() - before;
                    reread += 1;
                }
                continue;
            }
            const qint64 before = probeTimer.nsecsElapsed();
            (void) _bind(path);
            _emit(path);
            _probeCost[path] += probeTimer.nsecsElapsed() - before;
            unbound += 1;
        }
        _probePolls += 1;
        if (_probePolls >= 25) {
            QList<QPair<qint64, QString>> ranked;
            for (auto it = _probeCost.cbegin(); it != _probeCost.cend(); ++it) {
                ranked.append({ it.value(), it.key() });
            }
            std::sort(ranked.begin(), ranked.end(), [](const auto &a, const auto &b) { return a.first > b.first; });
            qint64 total = 0;
            for (const auto &entry : std::as_const(ranked)) {
                total += entry.first;
            }
            QString line = QStringLiteral("WATCHPROBE watched=%1 unbound=%2 reread=%3 total=%4ms")
                               .arg(_paths.size()).arg(unbound).arg(reread).arg(total / 1000000.0, 0, 'f', 1);
            for (int i = 0; i < ranked.size() && i < 8; ++i) {
                line += QStringLiteral(" | %1=%2ms").arg(ranked.at(i).second).arg(ranked.at(i).first / 1000000.0, 0, 'f', 1);
            }
            qWarning("%s", qPrintable(line));
            _probeCost.clear();
            _probePolls = 0;
        }
    }

    QHash<QString, qint64> _probeCost;
    int _probePolls = 0;"""

INCLUDE = "#include <QtCore/QElapsedTimer>\n"

def main():
    on = len(sys.argv) > 1 and sys.argv[1] == "on"
    text = SRC.read_text()
    if on:
        assert text.count(PLAIN) == 1, "poll body not in its plain shape"
        text = text.replace(PLAIN, PROBED)
        if INCLUDE not in text:
            text = text.replace("#include <QtCore/QTimer>", INCLUDE + "#include <QtCore/QTimer>", 1)
    else:
        assert text.count(PROBED) == 1, "poll body not instrumented"
        text = text.replace(PROBED, PLAIN)
        text = text.replace(INCLUDE, "", 1)
    SRC.write_text(text)
    print("watch probe", "on" if on else "off")

main()
