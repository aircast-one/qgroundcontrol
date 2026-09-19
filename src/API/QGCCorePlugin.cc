#include "QGCCorePlugin.h"
#include "AppSettings.h"
#ifdef Q_OS_ANDROID
#include "Viewer3DSettings.h"
#ifndef QGC_NO_SERIAL_LINK
#include "AndroidSerial.h"
#endif
#endif
#include "FactMetaData.h"
#include "FirmwarePluginManager.h"
#include "QGCMAVLink.h"
#include "HorizontalFactValueGrid.h"
#include "InstrumentValueData.h"
#include "JoystickManager.h"
#include "QGCLoggingCategory.h"
#include "QGCOptions.h"
#include "QmlComponentInfo.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "VideoReceiver.h"
#include "VideoBackend.h"
#include "SurveyPlanCreator.h"
#include "CorridorScanPlanCreator.h"
#include "StructureScanPlanCreator.h"
#include "SurveyComplexItem.h"
#include "CorridorScanComplexItem.h"
#include "StructureScanComplexItem.h"
#include "FixedWingLandingComplexItem.h"
#include "VTOLLandingComplexItem.h"
#include "Vehicle.h"
#include "BlankPlanCreator.h"
#include "ComplexMissionItem.h"
#include "PlanMasterController.h"

#ifdef QGC_CUSTOM_BUILD
#include CUSTOMHEADER
#endif

#include <QtCore/QApplicationStatic>
#include <QtCore/QFile>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
#include <QtQuick/QQuickItem>
#ifdef QGC_QWINDOWKIT
#include <QWKQuick/qwkquickglobal.h>
#ifdef QGC_GST_STREAMING
#include "GStreamer.h"
#endif
#endif

QGC_LOGGING_CATEGORY(QGCCorePluginLog, "API.QGCCorePlugin");

#ifndef QGC_CUSTOM_BUILD
Q_APPLICATION_STATIC(QGCCorePlugin, _qgcCorePluginInstance);
#endif

QGCCorePlugin::QGCCorePlugin(QObject *parent)
    : QObject(parent)
    , _defaultOptions(new QGCOptions(this))
    , _emptyCustomMapItems(new QmlObjectListModel(this))
{
    qCDebug(QGCCorePluginLog) << this;
}

QGCCorePlugin::~QGCCorePlugin()
{
    qCDebug(QGCCorePluginLog) << this;
}

QGCCorePlugin *QGCCorePlugin::instance()
{
#ifndef QGC_CUSTOM_BUILD
    return _qgcCorePluginInstance();
#else
    return CUSTOMCLASS::instance();
#endif
}

const QVariantList &QGCCorePlugin::analyzePages()
{
    // Log Viewer is excluded on mobile (Android/iOS) because parsing large log files
    // (e.g. 900 MB ULog files with 1000+ fields) exhausts the mobile heap, causing
    // OOM crashes. Proper mobile support requires time-bucketed downsampling and will
    // be addressed in a future major release.
#if defined(Q_OS_ANDROID) || defined(Q_OS_IOS)
    static const QVariantList analyzeList = {
#else
    static const QVariantList analyzeList = {
        QVariant::fromValue(new QmlComponentInfo(
            tr("Log Viewer"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/LogViewer/LogViewerPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/MAVLinkInspector.svg")))),
#endif
#ifndef Q_OS_ANDROID
        QVariant::fromValue(new QmlComponentInfo(
            tr("Onboard Logs"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/OnboardLogs/OnboardLogPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/OnboardLogIcon.svg")),
            nullptr, true /* requiresVehicle */)),
        QVariant::fromValue(new QmlComponentInfo(
            tr("GeoTag Images"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/GeoTag/GeoTagPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/GeoTag/GeoTagIcon.svg")))),
#endif
        QVariant::fromValue(new QmlComponentInfo(
            tr("MAVLink Console"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/MAVLinkConsole/MAVLinkConsolePage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/MAVLinkConsoleIcon.svg")),
            nullptr, true /* requiresVehicle */)),
        QVariant::fromValue(new QmlComponentInfo(
            tr("MAVLink Inspector"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/MAVLinkInspector/MAVLinkInspectorPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/MAVLinkInspector.svg")),
            nullptr, true /* requiresVehicle */)),
        QVariant::fromValue(new QmlComponentInfo(
            tr("Vibration"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/Vibration/VibrationPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/VibrationPageIcon")),
            nullptr, true /* requiresVehicle */)),
    };

    return analyzeList;
}

QGCOptions *QGCCorePlugin::options()
{
    return _defaultOptions;
}

const QmlObjectListModel *QGCCorePlugin::customMapItems()
{
    return _emptyCustomMapItems;
}

void QGCCorePlugin::adjustSettingMetaData(const QString &settingsGroup, FactMetaData &metaData, bool &userVisible)
{
#ifdef Q_OS_ANDROID
    // 3D view rendering is too flaky on Android GPUs/drivers; force the
    // feature off. Hiding the setting also forces it to its default value
    // (false) regardless of any previously saved user setting.
    if ((settingsGroup == Viewer3DSettings::settingsGroup) && (metaData.name() == Viewer3DSettings::enabledName)) {
        userVisible = false;
        return;
    }
#endif

    if (settingsGroup == AppSettings::settingsGroup) {
        if (metaData.name() == AppSettings::indoorPaletteName) {
#if defined(Q_OS_ANDROID)
            metaData.setRawDefaultValue(AppSettings::FollowSystemPalette);
#elif defined(Q_OS_IOS)
            metaData.setRawDefaultValue(0);
#else
            metaData.setRawDefaultValue(1);
#endif
            return;
        }
#ifndef Q_OS_ANDROID
        else if (metaData.name() == AppSettings::androidDontSaveToSDCardName) {
            userVisible = false;
            return;
        }
#endif
        else if (metaData.name() == AppSettings::androidUsePosixSerialName) {
#if defined(Q_OS_ANDROID) && !defined(QGC_NO_SERIAL_LINK)
            // Only show when the device actually exposes accessible serial device nodes
            userVisible = AndroidSerial::hasPosixSerialPorts();
#else
            userVisible = false;
#endif
            return;
        }
    }
}

QString QGCCorePlugin::showAdvancedUIMessage() const
{
    return tr("WARNING: You are about to enter Advanced Mode. "
              "If used incorrectly, this may cause your vehicle to malfunction thus voiding your warranty. "
              "You should do so only if instructed by customer support. "
              "Are you sure you want to enable Advanced Mode?");
}

bool QGCCorePlugin::showInitialSetupVehiclePreferences() const
{
    return !FirmwarePluginManager::instance()->singleVehicleSupport();
}

bool QGCCorePlugin::showInitialSetupMeasurementUnits() const
{
    return true;
}

void QGCCorePlugin::factValueGridCreateDefaultSettings(FactValueGrid* factValueGrid)
{
#if defined(Q_OS_ANDROID) || defined(Q_OS_IOS)
    FactValueGrid::FontSize defaultFontSize = FactValueGrid::DefaultFontSize;
#else
    FactValueGrid::FontSize defaultFontSize = FactValueGrid::MediumFontSize;
#endif

    if (factValueGrid->specificVehicleForCard()) {
        bool includeFWValues = factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassFixedWing || factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassVTOL || factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassAirship;

        factValueGrid->setFontSize(defaultFontSize);
        factValueGrid->appendColumn();
        factValueGrid->appendColumn();

        int rowIndex = 0;
        int colIndex = 0;

        // first cell
        QmlObjectListModel* column = factValueGrid->columns()->value<QmlObjectListModel*>(colIndex++);
        InstrumentValueData* value = column->value<InstrumentValueData*>(rowIndex);
        value->setFact("Vehicle", "AltitudeRelative");
        value->setIcon("arrow-thick-up.svg");
        value->setText(value->fact()->shortDescription());
        value->setShowUnits(true);

        // second cell
        column = factValueGrid->columns()->value<QmlObjectListModel*>(colIndex++);
        value = column->value<InstrumentValueData*>(rowIndex);
        if (includeFWValues) {
            value->setFact("Vehicle", "AirSpeed");
            value->setText("AirSpd");
            value->setShowUnits(true);
        } else {
            value->setFact("Vehicle", "GroundSpeed");
            value->setIcon("arrow-simple-right.svg");
            value->setText(value->fact()->shortDescription());
            value->setShowUnits(true);
        }
    } else {
        // DJI's flight telemetry set: distance, height, horizontal speed, vertical speed.
        // One value per chip, one row. Fixed wings additionally get airspeed, which is
        // stall-safety data rather than decoration.
        const bool includeFWValues = ((factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassFixedWing) ||
                                      (factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassVTOL) ||
                                      (factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassAirship));

        factValueGrid->setFontSize(defaultFontSize);

        struct DefaultValue {
            const char* factName;
            const char* icon;
            const char* text;       // empty: use the fact's own short description
        };
        const QList<DefaultValue> defaults = {
            { "DistanceToHome",   "home.svg",                "" },
            { "AltitudeRelative", "arrow-thick-up.svg",      "" },
            { "GroundSpeed",      "arrow-simple-right.svg",  "" },
            { "ClimbRate",        "arrow-simple-up.svg",     "" },
        };
        const QList<DefaultValue> fixedWingDefaults = {
            { "AirSpeed",         "",                        "AirSpd" },
        };

        for (const DefaultValue &def: (includeFWValues ? defaults + fixedWingDefaults : defaults)) {
            (void) factValueGrid->appendColumn();
            QmlObjectListModel *column = factValueGrid->columns()->value<QmlObjectListModel*>(factValueGrid->columns()->count() - 1);
            InstrumentValueData *value = column->value<InstrumentValueData*>(0);
            value->setFact(QStringLiteral("Vehicle"), QString::fromLatin1(def.factName));
            const QString icon = QString::fromLatin1(def.icon);
            if (!icon.isEmpty()) {
                value->setIcon(icon);
            }
            const QString text = QString::fromLatin1(def.text);
            // A fact the firmware does not publish leaves fact() null; the label is all we can
            // show for it, and crashing the whole default layout over one missing fact is worse.
            value->setText(!text.isEmpty()            ? text
                           : value->fact()            ? value->fact()->shortDescription()
                                                      : QString::fromLatin1(def.factName));
            value->setShowUnits(true);
        }
    }
}

QQmlApplicationEngine *QGCCorePlugin::createQmlApplicationEngine(QObject *parent)
{
    QQmlApplicationEngine *const qmlEngine = new QQmlApplicationEngine(parent);
    qmlEngine->addImportPath(QStringLiteral("qrc:/qml"));
#ifdef QGC_QWINDOWKIT
    QWK::registerTypes(qmlEngine);
#endif
    _setQmlContextProperties(qmlEngine);
    return qmlEngine;
}

void QGCCorePlugin::_setQmlContextProperties(QQmlEngine *qmlEngine)
{
    qmlEngine->rootContext()->setContextProperty(QStringLiteral("joystickManager"), JoystickManager::instance());
}

void QGCCorePlugin::setupEmbeddedEngine(QObject *rootObject)
{
    QQmlEngine *const engine = qmlEngine(rootObject);
    if (!engine) {
        return;
    }
    engine->addImportPath(QStringLiteral("qrc:/qml"));
    _setQmlContextProperties(engine);
}

namespace
{

bool s_hostProvidesPlanUI = false;

} // namespace

void QGCCorePlugin::setHostProvidesPlanUI(bool provides)
{
    s_hostProvidesPlanUI = provides;
}

bool QGCCorePlugin::hostProvidesPlanUI() const
{
    return s_hostProvidesPlanUI;
}

void QGCCorePlugin::destroyQmlApplicationEngine(QQmlApplicationEngine *qmlEngine)
{
    delete qmlEngine;
}

void QGCCorePlugin::createRootWindow(QQmlApplicationEngine *qmlEngine)
{
    qmlEngine->load(QUrl(QStringLiteral("qrc:/qml/QGroundControl/MainWindow.qml")));
}

VideoReceiver *QGCCorePlugin::createVideoReceiver(QObject *parent)
{
    return VideoBackend::createReceiver(parent);
}

void *QGCCorePlugin::createVideoSink(QQuickItem *widget, QObject *parent)
{
    return VideoBackend::createSink(widget, parent);
}
void QGCCorePlugin::releaseVideoSink(void *sink)
{
    VideoBackend::releaseSink(sink);
}

const QVariantList &QGCCorePlugin::toolBarIndicators()
{
    static const QVariantList toolBarIndicatorList = QVariantList(
        {
            QVariant::fromValue(QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/Toolbar/RTKGPSIndicator.qml"))),
            QVariant::fromValue(QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/Toolbar/GCSBatteryIndicator.qml"))),
        }
    );

    return toolBarIndicatorList;
}

QList<int> QGCCorePlugin::firstRunPromptStdIds()
{
    if (showInitialSetupVehiclePreferences() || showInitialSetupMeasurementUnits()) {
        return { kInitialSetupPromptId };
    }

    return {};
}

QVariantList QGCCorePlugin::firstRunPromptsToShow()
{
    QList<int> rgIdsToShow;

    rgIdsToShow.append(firstRunPromptStdIds());
    rgIdsToShow.append(firstRunPromptCustomIds());

    const QList<int> rgAlreadyShownIds = AppSettings::firstRunPromptsIdsVariantToList(SettingsManager::instance()->appSettings()->firstRunPromptIdsShown()->rawValue());
    for (int idToRemove: rgAlreadyShownIds) {
        (void) rgIdsToShow.removeOne(idToRemove);
    }

    QVariantList rgVarIdsToShow;
    for (int id: rgIdsToShow) {
        rgVarIdsToShow.append(id);
    }

    return rgVarIdsToShow;
}

QString QGCCorePlugin::firstRunPromptResource(int id) const
{
    switch (id) {
    case kInitialSetupPromptId:
        return QStringLiteral("/qml/QGroundControl/FirstRunPromptDialogs/InitialSetupPrompt.qml");
    default:
        return QString();
    }
}

void QGCCorePlugin::_setShowTouchAreas(bool show)
{
    if (show != _showTouchAreas) {
        _showTouchAreas = show;
        emit showTouchAreasChanged(show);
    }
}

void QGCCorePlugin::_setShowAdvancedUI(bool show)
{
    if (show != _showAdvancedUI) {
        _showAdvancedUI = show;
        emit showAdvancedUIChanged(show);
    }
}

QVariantList QGCCorePlugin::complexMissionItemNames(Vehicle *vehicle)
{
    auto makeEntry = [](const char* canonical, const QString& translated) {
        QVariantMap entry;
        entry[QStringLiteral("canonicalName")]  = QString(canonical);
        entry[QStringLiteral("translatedName")] = translated;
        return entry;
    };

    QVariantList items;
    items.append(makeEntry(SurveyComplexItem::canonicalName,       SurveyComplexItem::tr(SurveyComplexItem::canonicalName)));
    items.append(makeEntry(CorridorScanComplexItem::canonicalName, CorridorScanComplexItem::tr(CorridorScanComplexItem::canonicalName)));
    if (vehicle->multiRotor() || vehicle->vtol()) {
        items.append(makeEntry(StructureScanComplexItem::canonicalName, StructureScanComplexItem::tr(StructureScanComplexItem::canonicalName)));
    }
    // Note: Landing pattern items are not added here — they have their own dedicated button
    return items;
}

QList<PlanCreator*> QGCCorePlugin::planCreators(PlanMasterController *planMasterController)
{
    return {
        new SurveyPlanCreator(planMasterController),
        new CorridorScanPlanCreator(planMasterController),
        new StructureScanPlanCreator(planMasterController),
        new BlankPlanCreator(planMasterController),
    };
}

ComplexMissionItem *QGCCorePlugin::createComplexMissionItem(
    const QString &complexItemType,
    PlanMasterController *masterController,
    bool flyView,
    const QString &kmlOrShpFile)
{
    if (complexItemType == SurveyComplexItem::canonicalName || complexItemType == SurveyComplexItem::jsonComplexItemTypeValue) {
        return new SurveyComplexItem(masterController, flyView, kmlOrShpFile);
    } else if (complexItemType == CorridorScanComplexItem::canonicalName || complexItemType == CorridorScanComplexItem::jsonComplexItemTypeValue) {
        return new CorridorScanComplexItem(masterController, flyView, kmlOrShpFile);
    } else if (complexItemType == StructureScanComplexItem::canonicalName || complexItemType == StructureScanComplexItem::jsonComplexItemTypeValue) {
        return new StructureScanComplexItem(masterController, flyView, kmlOrShpFile);
    } else if (complexItemType == FixedWingLandingComplexItem::canonicalName || complexItemType == FixedWingLandingComplexItem::jsonComplexItemTypeValue) {
        return new FixedWingLandingComplexItem(masterController, flyView);
    } else if (complexItemType == VTOLLandingComplexItem::canonicalName || complexItemType == VTOLLandingComplexItem::jsonComplexItemTypeValue) {
        return new VTOLLandingComplexItem(masterController, flyView);
    }

    qCWarning(QGCCorePluginLog) << "QGCCorePlugin::createComplexMissionItem - Unknown complex item type:" << complexItemType;
    return nullptr;
}

void *QGCCorePlugin::createNativeVideoSink(QObject *parent)
{
#ifdef QGC_GST_STREAMING
    return GStreamer::createNativeSink(parent);
#else
    Q_UNUSED(parent);
    return nullptr;
#endif
}
