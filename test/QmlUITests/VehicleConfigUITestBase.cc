#include "VehicleConfigUITestBase.h"

#include <QtCore/QElapsedTimer>
#include <QtCore/QLocale>
#include <QtCore/QPointer>
#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "AppSettings.h"
#include "AutoPilotPlugin.h"
#include "Fact.h"
#include "ParameterManager.h"
#include "QGCApplication.h"
#include "SettingsManager.h"
#include "Vehicle.h"
#include "VehicleComponent.h"

namespace {

/// Returns true if the item tree (including \a item itself) contains at least
/// one visible text, control or image item with a non-zero size. Layout
/// containers report a size even when all their children are hidden, so only
/// leaf content items count.
bool hasVisibleContent(QQuickItem *item)
{
    if (!item->isVisible()) {
        return false;
    }
    if ((item->width() > 0) && (item->height() > 0) &&
        (item->inherits("QQuickText") || item->inherits("QQuickControl") || item->inherits("QQuickImageBase"))) {
        return true;
    }
    const QList<QQuickItem *> children = item->childItems();
    for (QQuickItem *child : children) {
        if (hasVisibleContent(child)) {
            return true;
        }
    }
    return false;
}

}

void VehicleConfigUITestBase::navigateToConfigureView()
{
    QVERIFY2(openVehicleSetup(), "Failed to open Vehicle Setup");
}

QQuickItem *VehicleConfigUITestBase::clickSidebarButton(const QString &objectName)
{
    QQuickItem *btn = findVisibleItem(_rootItem, objectName, 3000);
    if (!btn) {
        QTest::qFail(qPrintable(QStringLiteral("Sidebar button not found: %1").arg(objectName)), __FILE__, __LINE__);
        return nullptr;
    }
    if (!scrollIntoView(btn, QStringLiteral("setupSidebar")) || !_clickItemAt(btn, 0.5, 0.5, objectName)) {
        return nullptr;
    }
    if (!waitForCondition([btn] { return btn->property("checked").toBool(); }, TestTimeout::shortMs(),
                          QStringLiteral("%1 selected").arg(objectName))) {
        QTest::qFail(qPrintable(QStringLiteral("Sidebar button not selected after click: %1").arg(objectName)),
                     __FILE__, __LINE__);
        return nullptr;
    }
    return btn;
}

void VehicleConfigUITestBase::resetParamsToFirmwareDefaults(Vehicle *vehicle, const QString &sentinelParamName)
{
    ParameterManager *mgr = vehicle->parameterManager();

    Fact *sentinelFact = mgr->getParameter(ParameterManager::defaultComponentId, sentinelParamName);
    QVERIFY2(sentinelFact, qPrintable(QStringLiteral("%1 fact not found").arg(sentinelParamName)));
    QVERIFY2(sentinelFact->rawValue().toInt() != 0,
             qPrintable(QStringLiteral("%1 already at default before reset").arg(sentinelParamName)));

    mgr->resetAllParametersToDefaults();
    mgr->refreshAllParameters();

    QVERIFY2(QTest::qWaitFor([&] { return sentinelFact->rawValue().toInt() == 0; }, 30000),
             "Parameters never refreshed to firmware defaults");
}

void VehicleConfigUITestBase::resetAPMParamsToUncalibrated(Vehicle *vehicle)
{
    ParameterManager *mgr = vehicle->parameterManager();

    QVERIFY2(mgr->parameterExists(ParameterManager::defaultComponentId, QStringLiteral("COMPASS_OFS_X")),
             "COMPASS_OFS_X parameter not found");
    Fact *compassOfs = mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("COMPASS_OFS_X"));
    QVERIFY2(compassOfs, "COMPASS_OFS_X fact not found");

    mgr->resetAllParametersToDefaults();
    mgr->refreshAllParameters();

    QVERIFY2(QTest::qWaitFor([&] { return qFuzzyIsNull(compassOfs->rawValue().toFloat()); }, 30000),
             "COMPASS_OFS_X never refreshed to 0 after APM param reset");
}

void VehicleConfigUITestBase::clickThroughAllComponents(Vehicle *vehicle, const QString &vehicleName)
{
    const QString prefix = vehicleName.isEmpty() ? QString() : (vehicleName + QStringLiteral(": "));

    const QVariantList components = vehicle->autopilotPlugin()->vehicleComponents();
    QVERIFY2(!components.isEmpty(),
             qPrintable(QStringLiteral("%1No vehicle components found").arg(prefix)));

    for (const QVariant &compVariant : components) {
        auto *comp = compVariant.value<VehicleComponent *>();
        if (!comp) {
            continue;
        }

        const QString cleanName  = QString(comp->name()).remove(QLatin1Char(' '));
        const QString buttonName = QStringLiteral("setupComponent") + cleanName;

        QQuickItem *btn = findVisibleItem(_rootItem, buttonName, 2000);
        if (!btn) {
            continue;
        }

        clickSidebarButton(buttonName);
        if (QTest::currentTestFailed()) return;

        QQuickItem *loader = findVisibleItem(_rootItem, QStringLiteral("setupPanelLoader"), 2000);
        QVERIFY2(loader,
                 qPrintable(QStringLiteral("%1setupPanelLoader not found after clicking %2")
                                .arg(prefix, comp->name())));
        QVERIFY2(loader->property("item").value<QQuickItem *>() != nullptr,
                 qPrintable(QStringLiteral("%1Panel loader has no item after clicking %2")
                                .arg(prefix, comp->name())));

        verifyPanelContentVisible(prefix + comp->name());
        if (QTest::currentTestFailed()) return;
    }
}

void VehicleConfigUITestBase::verifyPanelContentVisible(const QString &context)
{
    QQuickItem *loader = findVisibleItem(_rootItem, QStringLiteral("setupPanelLoader"), 2000);
    QVERIFY2(loader, qPrintable(context + QStringLiteral(": setupPanelLoader not found")));

    QQuickItem *panel = loader->property("item").value<QQuickItem *>();
    QVERIFY2(panel, qPrintable(context + QStringLiteral(": panel loader has no item")));

    QQuickItem *target = panel;
    if (QQuickItem *contentLoader = panel->findChild<QQuickItem *>(QStringLiteral("setupPage_contentLoader"))) {
        QQuickItem *contentItem = contentLoader->property("item").value<QQuickItem *>();
        QVERIFY2(contentItem, qPrintable(context + QStringLiteral(": setup page content loader has no item")));
        target = contentItem;
    }

    QVERIFY2(QTest::qWaitFor([&] { return hasVisibleContent(target); }, 3000),
             qPrintable(context + QStringLiteral(": page loaded but renders no visible content (blank page)")));
}

void VehicleConfigUITestBase::clickThroughAllComponentsAllLocales(Vehicle *vehicle, const QString &vehicleName)
{
    Fact *languageFact = SettingsManager::instance()->appSettings()->qLocaleLanguage();
    const QVariant savedLanguage = languageFact->rawValue();

    auto restoreLanguage = qScopeGuard([this, languageFact, savedLanguage] {
        if (languageFact->rawValue() == savedLanguage) {
            return;
        }
        qgcApp()->resetRebootMessageDebounce();
        languageFact->setRawValue(savedLanguage);
        (void) acceptDialog(5000);
    });

    ignoreLogMessage("API.QGCApplication", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Qt lib localization for .* is not present")));
    ignoreLogMessage("API.QGCApplication", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Error loading source localization for .*")));
    ignoreLogMessage("API.QGCApplication", QtWarningMsg,
                     QRegularExpression(QStringLiteral("Error loading json localization for .*")));

    if (languageFact->rawValue().toInt() != QLocale::English) {
        expectAppMessage(QRegularExpression(QStringLiteral("Restart application for changes to take effect")));
        languageFact->setRawValue(QLocale::English);
        QVERIFY2(acceptDialog(5000), "Restart-required dialog never shown after switching language to English");
        verifyExpectedLogMessage();
        if (QTest::currentTestFailed()) return;
    }

    clickThroughAllComponents(vehicle, vehicleName);
    if (QTest::currentTestFailed()) return;

    clickSidebarButton(QStringLiteral("setupComponentSensors"));
    if (QTest::currentTestFailed()) return;
    QPointer<QQuickItem> anchor = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_calibrateCompass"), 3000);
    QVERIFY2(anchor, "Calibrate Compass language anchor not found");
    const QString englishAnchorText = anchor->property("text").toString();

    qgcApp()->resetRebootMessageDebounce();
    expectAppMessage(QRegularExpression(QStringLiteral("Restart application for changes to take effect")));
    languageFact->setRawValue(QLocale::Chinese);
    QVERIFY2(acceptDialog(5000), "Restart-required dialog never shown after switching language to Chinese");
    verifyExpectedLogMessage();
    if (QTest::currentTestFailed()) return;

    QVERIFY2(anchor, "Language anchor destroyed during language switch");
    QVERIFY2(QTest::qWaitFor([&] { return anchor && (anchor->property("text").toString() != englishAnchorText); }, 3000),
             qPrintable(QStringLiteral("UI text did not change after switching to Chinese (still '%1')")
                            .arg(englishAnchorText)));

    const QString zhPrefix = vehicleName.isEmpty() ? QStringLiteral("zh_CN") : vehicleName + QStringLiteral(" zh_CN");
    clickThroughAllComponents(vehicle, zhPrefix);
    if (QTest::currentTestFailed()) return;

    if (languageFact->rawValue() != savedLanguage) {
        qgcApp()->resetRebootMessageDebounce();
        expectAppMessage(QRegularExpression(QStringLiteral("Restart application for changes to take effect")));
        languageFact->setRawValue(savedLanguage);
        QVERIFY2(acceptDialog(5000), "Restart-required dialog never shown after restoring language");
        verifyExpectedLogMessage();
    }

    if (savedLanguage.toInt() == QLocale::English) {
        QPointer<QQuickItem> restoredAnchor = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_calibrateCompass"), 3000);
        if (restoredAnchor) {
            QVERIFY2(QTest::qWaitFor([&] { return restoredAnchor && (restoredAnchor->property("text").toString() == englishAnchorText); }, 3000),
                     "UI text did not return to English after restoring language");
        }
    }
}

void VehicleConfigUITestBase::waitForParamRefreshQuiet(Vehicle *vehicle)
{
    ParameterManager *mgr = vehicle->parameterManager();
    QElapsedTimer sinceLastResponse;
    sinceLastResponse.start();

    QObject context;
    QObject::connect(mgr, &ParameterManager::_paramRequestReadSuccess, &context,
                     [&] { sinceLastResponse.restart(); });
    QObject::connect(mgr, &ParameterManager::_paramRequestReadFailure, &context,
                     [&] { sinceLastResponse.restart(); });

    QVERIFY2(QTest::qWaitFor([&] { return sinceLastResponse.elapsed() > 500; }, 10000),
             "waitForParamRefreshQuiet: parameter refresh traffic still active after 10s");
}

void VehicleConfigUITestBase::navigateToAPMSensorsPage()
{
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    QQuickItem *sensorsBtn = clickSidebarButton(QStringLiteral("setupComponentSensors"));
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_calibrateAccel"), 5000),
             "sensorsSetup_calibrateAccel not found after opening Sensors page");
    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_calibrateCompass"), 3000),
             "sensorsSetup_calibrateCompass not found after opening Sensors page");
    Q_UNUSED(sensorsBtn);
}

void VehicleConfigUITestBase::verifyAPMCalIndicators(bool compassGreen, bool accelGreen, const char *context)
{
    struct Check {
        const char *name;
        bool expectedCalibrated;
    };
    const Check checks[] = {
        { "sensorsSetup_calibrateCompass", compassGreen },
        { "sensorsSetup_calibrateAccel",   accelGreen   },
    };

    for (const Check &c : checks) {
        QPointer<QQuickItem> row = findVisibleItem(_rootItem, QLatin1String(c.name), 3000);
        QVERIFY2(row, qPrintable(QStringLiteral("Calibration row not found (%1): %2")
                                     .arg(QLatin1String(context), QLatin1String(c.name))));
        const bool calibrated = waitForCondition(
            [&] { return row && (row->property("needsCalibration").toBool() != c.expectedCalibrated); },
            TestTimeout::mediumMs(), QStringLiteral("%1 calibration state").arg(QLatin1String(c.name)));
        QVERIFY2(calibrated, qPrintable(QStringLiteral("Calibrated state is not %1 (%2): %3")
                                            .arg(c.expectedCalibrated)
                                            .arg(QLatin1String(context), QLatin1String(c.name))));
    }
}

void VehicleConfigUITestBase::runAPMFullAccelCal()
{
    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_calibrateAccel")),
             "Failed to click Accelerometer button");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 5000),
             "Pre-accel-cal dialog accept button not found");
    QVERIFY2(acceptDialog(),
             "Failed to accept pre-accel-cal dialog");

    struct PoseInfo {
        const char *sideObjectName;
    };
    static constexpr PoseInfo kPoses[] = {
        { "sensorsCal_downSide"      },
        { "sensorsCal_leftSide"      },
        { "sensorsCal_rightSide"     },
        { "sensorsCal_noseDownSide"  },
        { "sensorsCal_tailDownSide"  },
        { "sensorsCal_upsideDownSide" },
    };

    for (const PoseInfo &pose : kPoses) {
        QPointer<QQuickItem> sideItem = findVisibleItem(_rootItem, QLatin1String(pose.sideObjectName), 10000);
        QVERIFY2(sideItem, qPrintable(QStringLiteral("Side indicator not visible: %1")
                                          .arg(QLatin1String(pose.sideObjectName))));

        QVERIFY2(QTest::qWaitFor([&] { return sideItem && (sideItem->property("calState").toInt() == 2); }, 10000),
                 qPrintable(QStringLiteral("Side never went InProgress: %1")
                                .arg(QLatin1String(pose.sideObjectName))));

        QPointer<QQuickItem> nextBtn = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_nextButton"), 3000);
        QVERIFY2(nextBtn, "Next button not found");
        QVERIFY2(QTest::qWaitFor([&] { return nextBtn && nextBtn->property("enabled").toBool(); }, 3000),
                 qPrintable(QStringLiteral("Next button not enabled for side: %1")
                                .arg(QLatin1String(pose.sideObjectName))));
        QVERIFY2(clickButton(QStringLiteral("sensorsSetup_nextButton")), "Failed to click Next button");
    }

    QPointer<QQuickItem> progressBar = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"), 2000);
    QVERIFY2(progressBar, "Progress bar not found during accel cal");
    QVERIFY2(QTest::qWaitFor([&] { return progressBar && qFuzzyCompare(progressBar->property("value").toDouble(), 1.0); }, 10000),
             "Progress bar never reached 1.0 after accel cal success");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("postCalibrationDialog"), 5000),
             "Post-accel-cal dialog not shown");
    QVERIFY2(waitForCondition([&] { return progressBar && !progressBar->isVisible(); }, TestTimeout::shortMs(),
                              QStringLiteral("calibration sheet closed")),
             "Calibration sheet still covers the post-accel-cal dialog");
    QVERIFY2(!_rootItem->findChild<QQuickItem*>(QStringLiteral("postOnboardCompassCalibrationDialog")),
             "Compass results dialog incorrectly shown after accel cal");
    QVERIFY2(acceptDialog(),
             "Failed to dismiss post-accel-cal dialog");
}

void VehicleConfigUITestBase::runAPMCompassCal()
{
    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_calibrateCompass")),
             "Failed to click Compass button");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 5000),
             "Pre-compass-cal dialog accept button not found");
    QVERIFY2(acceptDialog(),
             "Failed to accept pre-compass-cal dialog");

    QPointer<QQuickItem> progressBar = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"), 2000);
    QVERIFY2(progressBar, "Progress bar not found during compass cal");

    QVERIFY2(QTest::qWaitFor([&] { return progressBar && (progressBar->property("value").toDouble() > 0.0); }, 5000),
             "Compass cal progress never advanced above 0");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("postOnboardCompassCalibrationDialog"), 25000),
             "Post-compass-cal dialog not shown");
    QVERIFY2(waitForCondition([&] { return progressBar && !progressBar->isVisible(); }, TestTimeout::shortMs(),
                              QStringLiteral("calibration sheet closed")),
             "Calibration sheet still covers the post-compass-cal dialog");
    QVERIFY2(acceptDialog(),
             "Failed to dismiss post-compass-cal dialog");
}
