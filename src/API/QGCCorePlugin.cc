/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCCorePlugin.h"
#include "QGCLogging.h"
#include "AppSettings.h"
#include "MavlinkSettings.h"
#include "FactMetaData.h"
#ifdef QGC_GST_STREAMING
#include "GStreamer.h"
#endif
#include "HorizontalFactValueGrid.h"
#include "InstrumentValueData.h"
#include "JoystickManager.h"
#include "LogDownloadController.h"
#include "MAVLinkLib.h"
#include "QGCLoggingCategory.h"
#include "QGCOptions.h"
#include "QmlComponentInfo.h"
#include "QmlObjectListModel.h"
#ifdef QGC_QT_STREAMING
#include "QtMultimediaReceiver.h"
#endif
#include "SettingsManager.h"
#include "VideoReceiver.h"

#ifdef QGC_CUSTOM_BUILD
#include CUSTOMHEADER
#endif

#include <QtCore/qapplicationstatic.h>
#include <QtCore/QFile>
#include <QtQml/qqml.h>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
#include <QtQuick/QQuickItem>
#ifdef QGC_QWINDOWKIT
#include <QWKQuick/qwkquickglobal.h>
#endif

QGC_LOGGING_CATEGORY(QGCCorePluginLog, "qgc.api.qgccoreplugin");

#ifndef QGC_CUSTOM_BUILD
Q_APPLICATION_STATIC(QGCCorePlugin, _qgcCorePluginInstance);
#endif

QGCCorePlugin::QGCCorePlugin(QObject *parent)
    : QObject(parent)
    , _defaultOptions(new QGCOptions(this))
    , _emptyCustomMapItems(new QmlObjectListModel(this))
{
    // qCDebug(QGCCorePluginLog) << Q_FUNC_INFO << this;
}

QGCCorePlugin::~QGCCorePlugin()
{
    // qCDebug(QGCCorePluginLog) << Q_FUNC_INFO << this;
}

QGCCorePlugin *QGCCorePlugin::instance()
{
#ifndef QGC_CUSTOM_BUILD
    return _qgcCorePluginInstance();
#else
    return CUSTOMCLASS::instance();
#endif
}

void QGCCorePlugin::registerQmlTypes()
{
    (void) qmlRegisterUncreatableType<QGCCorePlugin>("QGroundControl", 1, 0, "QGCCorePlugin", QStringLiteral("Reference only"));
    (void) qmlRegisterUncreatableType<QGCOptions>("QGroundControl", 1, 0, "QGCOptions", QStringLiteral("Reference only"));
    (void) qmlRegisterUncreatableType<QGCFlyViewOptions>("QGroundControl", 1, 0, "QGCFlyViewOptions", QStringLiteral("Reference only"));
}

const QVariantList &QGCCorePlugin::analyzePages()
{
    static const QVariantList analyzeList = {
        QVariant::fromValue(new QmlComponentInfo(
            tr("Log Download"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/LogDownloadPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/LogDownloadIcon.svg")))),
#if !defined(Q_OS_ANDROID) && !defined(Q_OS_IOS)
        QVariant::fromValue(new QmlComponentInfo(
            tr("GeoTag Images"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/GeoTagPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/GeoTagIcon.svg")))),
#endif
#ifndef Q_OS_ANDROID
        QVariant::fromValue(new QmlComponentInfo(
            tr("MAVLink Console"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/MAVLinkConsolePage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/MAVLinkConsoleIcon.svg")))),
#endif
#if !defined(QGC_DISABLE_MAVLINK_INSPECTOR) && !defined(Q_OS_ANDROID)
        QVariant::fromValue(new QmlComponentInfo(
            tr("MAVLink Inspector"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/MAVLinkInspectorPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/MAVLinkInspector.svg")))),
#endif
#ifndef Q_OS_ANDROID
        QVariant::fromValue(new QmlComponentInfo(
            tr("Vibration"),
            QUrl::fromUserInput(QStringLiteral("qrc:/qml/QGroundControl/AnalyzeView/VibrationPage.qml")),
            QUrl::fromUserInput(QStringLiteral("qrc:/qmlimages/VibrationPageIcon")))),
#endif
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

bool QGCCorePlugin::adjustSettingMetaData(const QString &settingsGroup, FactMetaData &metaData)
{
    if (settingsGroup == AppSettings::settingsGroup) {
        if (metaData.name() == AppSettings::indoorPaletteName) {
#if defined(Q_OS_ANDROID)
            metaData.setRawDefaultValue(AppSettings::FollowSystemPalette);
#elif defined(Q_OS_IOS)
            metaData.setRawDefaultValue(0);
#else
            metaData.setRawDefaultValue(1);
#endif
            return true;
        }
#if defined(Q_OS_ANDROID) || defined(Q_OS_IOS)
        else if (metaData.name() == MavlinkSettings::telemetrySaveName) {
            metaData.setRawDefaultValue(false);
            return true;
        }
#endif
#ifndef Q_OS_ANDROID
        else if (metaData.name() == AppSettings::androidSaveToSDCardName) {
            return false;
        }
#endif
    }

    return true;
}

QString QGCCorePlugin::showAdvancedUIMessage() const
{
    return tr("WARNING: You are about to enter Advanced Mode. "
              "If used incorrectly, this may cause your vehicle to malfunction thus voiding your warranty. "
              "You should do so only if instructed by customer support. "
              "Are you sure you want to enable Advanced Mode?");
}

void QGCCorePlugin::factValueGridCreateDefaultSettings(FactValueGrid* factValueGrid)
{
    if (factValueGrid->specificVehicleForCard()) {
        bool includeFWValues = factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassFixedWing || factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassVTOL || factValueGrid->vehicleClass() == QGCMAVLink::VehicleClassAirship;

        factValueGrid->setFontSize(FactValueGrid::LargeFontSize);
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

        factValueGrid->setFontSize(FactValueGrid::LargeFontSize);

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
    qmlEngine->rootContext()->setContextProperty(QStringLiteral("debugMessageModel"), QGCLogging::instance());
    qmlEngine->rootContext()->setContextProperty(QStringLiteral("logDownloadController"), LogDownloadController::instance());
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

void QGCCorePlugin::createRootWindow(QQmlApplicationEngine *qmlEngine)
{
    qmlEngine->load(QUrl(QStringLiteral("qrc:/qml/QGroundControl/MainWindow/MainWindow.qml")));
}

VideoReceiver *QGCCorePlugin::createVideoReceiver(QObject *parent)
{
#ifdef QGC_GST_STREAMING
    return GStreamer::createVideoReceiver(parent);
#elif defined(QGC_QT_STREAMING)
    return QtMultimediaReceiver::createVideoReceiver(parent);
#else
    return nullptr;
#endif
}

void *QGCCorePlugin::createVideoSink(QQuickItem *widget, QObject *parent)
{
#ifdef QGC_GST_STREAMING
    return GStreamer::createVideoSink(widget, parent);
#elif defined(QGC_QT_STREAMING)
    return QtMultimediaReceiver::createVideoSink(widget, parent);
#else
    Q_UNUSED(widget); Q_UNUSED(parent);
    return nullptr;
#endif
}
void QGCCorePlugin::setVideoSinkWidget(void *sink, QQuickItem *widget)
{
#ifdef QGC_GST_STREAMING
    GStreamer::setVideoSinkWidget(sink, widget);
#else
    Q_UNUSED(sink); Q_UNUSED(widget);
    qCWarning(QGCCorePluginLog) << "setVideoSinkWidget not supported by this video backend";
#endif
}

void QGCCorePlugin::releaseVideoSink(void *sink)
{
#ifdef QGC_GST_STREAMING
    GStreamer::releaseVideoSink(sink);
#elif defined(QGC_QT_STREAMING)
    QtMultimediaReceiver::releaseVideoSink(sink);
#else
    Q_UNUSED(sink);
#endif
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
    case kUnitsFirstRunPromptId:
        return QStringLiteral("/FirstRunPromptDialogs/UnitsFirstRunPrompt.qml");
    case kOfflineVehicleFirstRunPromptId:
        return QStringLiteral("/FirstRunPromptDialogs/OfflineVehicleFirstRunPrompt.qml");
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
