#include "ToolbarIndicatorUITest.h"

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "Fact.h"
#include "MockLink.h"
#include "RemoteIDSettings.h"
#include "SettingsManager.h"

#include <QtCore/QPointer>
#include <QtCore/QScopeGuard>

UT_REGISTER_TEST(ToolbarIndicatorUITest, TestLabel::Integration)

bool ToolbarIndicatorUITest::_exerciseIndicator(QQuickItem *indicatorItem, const QString &indicatorName, bool expectExpand)
{
    if (!indicatorItem || !_window) {
        return false;
    }

    if (!waitForCondition([indicatorItem] { return indicatorItem->width() > 0; }, TestTimeout::mediumMs(),
                          QStringLiteral("%1 has a size").arg(indicatorName))
        || !_clickItemAt(indicatorItem, 0.5, 0.5, indicatorName)) {
        return false;
    }
    if (!findVisibleItem(_rootItem, QStringLiteral("drawerLoader"), 2000)) {
        qWarning() << indicatorName << ": drawer did not open after clicking indicator";
        return false;
    }

    if (expectExpand) {
        QQuickItem *expandBtn = findVisibleItem(_rootItem, QStringLiteral("drawerDetailsRow"), 500);
        if (!expandBtn) {
            qWarning() << indicatorName << ": expand button not found but was expected";
            return false;
        }
        if (!_clickItemAt(expandBtn, 0.5, 0.5, indicatorName)) {
            return false;
        }

        if (!findVisibleItem(_rootItem, QStringLiteral("indicatorExpandedLoader"), 2000)) {
            qWarning() << indicatorName << ": expanded content did not appear after clicking expand button";
            return false;
        }
    }

    QTest::keyClick(_window, Qt::Key_Escape);

    const bool drawerClosed = waitForCondition(
        [&] { return findVisibleItem(_rootItem, QStringLiteral("drawerLoader"), 0) == nullptr; },
        2000,
        QStringLiteral("drawerLoader hidden"));
    if (!drawerClosed) {
        qWarning() << indicatorName << ": drawer did not close after pressing Escape";
        return false;
    }

    return true;
}

void ToolbarIndicatorUITest::_runIndicatorTest(
    const std::function<MockLink *()> &factory,
    const QString &vehicleName)
{
    runWithMockLink(factory, [&](QPointer<MockLink> /*mockLink*/, Vehicle * /*vehicle*/) {
    Fact *const sendBasicID = SettingsManager::instance()->remoteIDSettings()->sendBasicID();
    const QVariant savedSendBasicID = sendBasicID->rawValue();
    const auto restoreSendBasicID = qScopeGuard([sendBasicID, savedSendBasicID] { sendBasicID->setRawValue(savedSendBasicID); });
    sendBasicID->setRawValue(true);
    QVERIFY2(_window->property("flyViewActive").toBool(),
             qPrintable(QStringLiteral("%1: Fly view not active").arg(vehicleName)));

    struct IndicatorSpec {
        const char *objectName;
        const char *displayName;
        bool        expectExpand;
    };
    static const IndicatorSpec kIndicators[] = {
        { "mainStatusPill",                 "MainStatus",   true  },
        { "flightModeIndicator",            "FlightMode",   true  },
        { "toolbar_gpsIndicator",           "GPS",          true  },
        { "indicatorSlotBatteryIndicator",  "Battery",      true  },
        { "toolbar_remoteIDIndicator",      "RemoteID",     true  },
        { "toolbar_gimbalIndicator",        "Gimbal",       true  },
        { "toolbar_escIndicator",           "ESC",          false },
        { "toolbar_telemetryRSSIIndicator", "TelemetryRSSI",false },
    };

    for (const IndicatorSpec &spec : kIndicators) {
        const QString objName     = QString::fromLatin1(spec.objectName);
        const QString displayName = vehicleName + QLatin1Char('/') + QLatin1String(spec.displayName);

        QQuickItem *item = findVisibleItem(_rootItem, objName, 2000);
        QVERIFY2(item,
                 qPrintable(QStringLiteral("%1: %2 not found in toolbar").arg(vehicleName, objName)));
        QVERIFY2(_exerciseIndicator(item, displayName, spec.expectExpand),
                 qPrintable(QStringLiteral("%1: exercise failed").arg(displayName)));
    }
    });
}

void ToolbarIndicatorUITest::_testPX4Indicators()
{
    _runIndicatorTest(
        [] { return MockLink::startPX4MockLink(MockConfiguration::OptionEnableGimbal); },
        QStringLiteral("PX4"));
}

void ToolbarIndicatorUITest::_testAPMCopterIndicators()
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }

    _runIndicatorTest(
        [] { return MockLink::startAPMArduCopterMockLink(MockConfiguration::OptionEnableGimbal); },
        QStringLiteral("APMCopter"));
}
