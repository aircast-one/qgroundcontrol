/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/


/**
 * @file
 *   @brief Implementation of class QGCApplication
 *
 *   @author Lorenz Meier <mavteam@student.ethz.ch>
 *
 */

#include "QGCApplication.h"

#include <QtCore/QEvent>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaMethod>
#include <QtCore/QMetaObject>
#include <QtCore/QRegularExpression>
#include <QtCore/QUrlQuery>
#include <QtGui/QFileOpenEvent>
#include <QtGui/QFontDatabase>
#include <QtGui/QIcon>
#include <QtGui/QStyleHints>
#include <QtNetwork/QHostInfo>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkProxyFactory>
#include <QtNetwork/QNetworkReply>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
#include <QtQuick/QQuickImageProvider>
#include <QtQuick/QQuickWindow>
#include <QtQuickControls2/QQuickStyle>

#include <QtCore/private/qthread_p.h>

#include "QGCLogging.h"
#include "AudioOutput.h"
#include "AutoPilotPlugin.h"
#include "CmdLineOptParser.h"
#include "DebugApiServer.h"
#include "ESP8266ComponentController.h"
#include "FollowMe.h"
#include "GeoTagController.h"
#include "GimbalController.h"
#include "GPSRtk.h"
#include "JoystickConfigController.h"
#include "JoystickManager.h"
#include "JsonHelper.h"
#include "LinkManager.h"
#include "LogDownloadController.h"
#include "MAVLinkChartController.h"
#include "MAVLinkConsoleController.h"
#include "MAVLinkProtocol.h"
#include "MissionManager.h"
#include "MultiVehicleManager.h"
#ifdef QGC_WFB_ENABLED
#include "PacketRadioManager.h"
#endif
#include "ParameterManager.h"
#include "PositionManager.h"
#include "QGCCameraManager.h"
#include "QGCCorePlugin.h"
#include "QGCFileDownload.h"
#include "QGCImageProvider.h"
#include "QGCLoggingCategory.h"
#include "QGroundControlQmlGlobal.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "AppSettings.h"
#include "VideoSettings.h"
#include "ShapeFileHelper.h"
#include "SyslinkComponentController.h"
#include "TCPLink.h"
#include "UDPLink.h"
#include "Vehicle.h"
#include "VehicleComponent.h"
#include "VideoManager.h"

#ifndef QGC_DISABLE_MAVLINK_INSPECTOR
#include "MAVLinkInspectorController.h"
#include "OverlayPhysics.h"
#endif
#ifdef QGC_VIEWER3D
#include "Viewer3DManager.h"
#endif
#ifndef QGC_NO_SERIAL_LINK
#include "FirmwareUpgradeController.h"
#include "SerialLink.h"
#endif

#ifdef Q_OS_LINUX
#ifndef Q_OS_ANDROID
#include <unistd.h>
#include <sys/types.h>
#endif
#endif

QGC_LOGGING_CATEGORY(QGCApplicationLog, "qgc.qgcapplication")

// Qml Singleton factories

static QObject *mavlinkSingletonFactory(QQmlEngine*, QJSEngine*)
{
    return new QGCMAVLink();
}

QGCApplication::QGCApplication(int &argc, char *argv[], bool unitTesting, bool simpleBootTest)
    : QApplication(argc, argv)
    , _runningUnitTests(unitTesting)
    , _simpleBootTest(simpleBootTest)
{
    _msecsElapsedTime.start();

    // Setup for network proxy support
    QNetworkProxyFactory::setUseSystemConfiguration(true);

    // Parse command line options
    bool fClearSettingsOptions = false; // Clear stored settings
    bool fClearCache = false;           // Clear parameter/airframe caches
    bool logging = false;               // Turn on logging
    QString loggingOptions;

    CmdLineOpt_t rgCmdLineOptions[] = {
        { "--clear-settings",   &fClearSettingsOptions, nullptr },
        { "--clear-cache",      &fClearCache,           nullptr },
        { "--logging",          &logging,               &loggingOptions },
        { "--fake-mobile",      &_fakeMobile,           nullptr },
        { "--log-output",       &_logOutput,            nullptr },
        // Add additional command line option flags here
    };

    ParseCmdLineOptions(argc, argv, rgCmdLineOptions, std::size(rgCmdLineOptions), false);

    // Set up timer for delayed missing fact display
    _missingParamsDelayedDisplayTimer.setSingleShot(true);
    _missingParamsDelayedDisplayTimer.setInterval(_missingParamsDelayedDisplayTimerTimeout);
    (void) connect(&_missingParamsDelayedDisplayTimer, &QTimer::timeout, this, &QGCApplication::_missingParamsDisplay);

    // Set application information
    QString applicationName;
    if (_runningUnitTests || simpleBootTest) {
        // We don't want unit tests to use the same QSettings space as the normal app. So we tweak the app
        // name. Also we want to run unit tests with clean settings every time.
        applicationName = QStringLiteral("%1_unittest").arg(QGC_APP_NAME);
    } else {
#ifdef QGC_DAILY_BUILD
        // This gives daily builds their own separate settings space. Allowing you to use daily and stable builds
        // side by side without daily screwing up your stable settings.
        applicationName = QStringLiteral("%1 Daily").arg(QGC_APP_NAME);
#else
        applicationName = QGC_APP_NAME;
#endif
    }
    setApplicationName(applicationName);
    setOrganizationName(QGC_ORG_NAME);
    setOrganizationDomain(QGC_ORG_DOMAIN);
    setApplicationVersion(QString(QGC_APP_VERSION_STR));
    styleHints()->setMousePressAndHoldInterval(500);
#ifdef Q_OS_LINUX
    setWindowIcon(QIcon(":/res/qgroundcontrol.ico"));
#endif

    // Set settings format
    QSettings::setDefaultFormat(QSettings::IniFormat);
    QSettings settings;
    qCDebug(QGCApplicationLog) << "Settings location" << settings.fileName() << "Is writable?:" << settings.isWritable();

    if (!settings.isWritable()) {
        qCWarning(QGCApplicationLog) << "Setings location is not writable";
    }

    // The setting will delete all settings on this boot
    fClearSettingsOptions |= settings.contains(_deleteAllSettingsKey);

    if (_runningUnitTests || simpleBootTest) {
        // Unit tests run with clean settings
        fClearSettingsOptions = true;
    }

    if (fClearSettingsOptions) {
        // User requested settings to be cleared on command line
        settings.clear();

        // Clear parameter cache
        QDir paramDir(ParameterManager::parameterCacheDir());
        paramDir.removeRecursively();
        paramDir.mkpath(paramDir.absolutePath());
    } else {
        // Determine if upgrade message for settings version bump is required. Check and clear must happen before toolbox is started since
        // that will write some settings.
        if (settings.contains(_settingsVersionKey)) {
            if (settings.value(_settingsVersionKey).toInt() != QGC_SETTINGS_VERSION) {
                settings.clear();
                _settingsUpgraded = true;
            }
        }
    }
    settings.setValue(_settingsVersionKey, QGC_SETTINGS_VERSION);

    if (fClearCache) {
        QDir dir(ParameterManager::parameterCacheDir());
        dir.removeRecursively();
        QFile airframe(cachedAirframeMetaDataFile());
        airframe.remove();
        QFile parameter(cachedParameterMetaDataFile());
        parameter.remove();
    }

    // Set up our logging filters
    QGCLoggingCategoryRegister::instance()->setFilterRulesFromSettings(loggingOptions);

    // We need to set language as early as possible prior to loading on JSON files.
    setLanguage();

#ifndef QGC_DAILY_BUILD
    _checkForNewVersion();
#endif
}

void QGCApplication::setLanguage()
{
    _locale = QLocale::system();
    qCDebug(QGCApplicationLog) << "System reported locale:" << _locale << "; Name" << _locale.name() << "; Preffered (used in maps): " << (QLocale::system().uiLanguages().length() > 0 ? QLocale::system().uiLanguages()[0] : "None");

    QLocale::Language possibleLocale = AppSettings::_qLocaleLanguageEarlyAccess();
    if (possibleLocale != QLocale::AnyLanguage) {
        _locale = QLocale(possibleLocale);
    }
    //-- We have specific fonts for Korean
    if (_locale == QLocale::Korean) {
        qCDebug(QGCApplicationLog) << "Loading Korean fonts" << _locale.name();
        if(QFontDatabase::addApplicationFont(":/fonts/NanumGothic-Regular") < 0) {
            qCWarning(QGCApplicationLog) << "Could not load /fonts/NanumGothic-Regular font";
        }
        if(QFontDatabase::addApplicationFont(":/fonts/NanumGothic-Bold") < 0) {
            qCWarning(QGCApplicationLog) << "Could not load /fonts/NanumGothic-Bold font";
        }
    }
    qCDebug(QGCApplicationLog) << "Loading localizations for" << _locale.name();
    removeTranslator(JsonHelper::translator());
    removeTranslator(&_qgcTranslatorSourceCode);
    removeTranslator(&_qgcTranslatorQtLibs);
    if (_locale.name() != "en_US") {
        QLocale::setDefault(_locale);
        if (_qgcTranslatorQtLibs.load("qt_" + _locale.name(), QLibraryInfo::path(QLibraryInfo::TranslationsPath))) {
            installTranslator(&_qgcTranslatorQtLibs);
        } else {
            qCWarning(QGCApplicationLog) << "Qt lib localization for" << _locale.name() << "is not present";
        }
        if (_qgcTranslatorSourceCode.load(_locale, QLatin1String("qgc_source_"), "", ":/i18n")) {
            installTranslator(&_qgcTranslatorSourceCode);
        } else {
            qCWarning(QGCApplicationLog) << "Error loading source localization for" << _locale.name();
        }
        if (JsonHelper::translator()->load(_locale, QLatin1String("qgc_json_"), "", ":/i18n")) {
            installTranslator(JsonHelper::translator());
        } else {
            qCWarning(QGCApplicationLog) << "Error loading json localization for" << _locale.name();
        }
    }

    if (_qmlAppEngine) {
        _qmlAppEngine->retranslate();
    }

    emit languageChanged(_locale);
}

QGCApplication::~QGCApplication()
{

}

void QGCApplication::init()
{
    SettingsManager::instance()->init();

    LinkManager::registerQmlTypes();
    ParameterManager::registerQmlTypes();
    QGroundControlQmlGlobal::registerQmlTypes();
    MissionManager::registerQmlTypes();
    QGCCameraManager::registerQmlTypes();
    MultiVehicleManager::registerQmlTypes();
    QGCPositionManager::registerQmlTypes();
    SettingsManager::registerQmlTypes();
    VideoManager::registerQmlTypes();
    QGCCorePlugin::registerQmlTypes();
    GPSRtk::registerQmlTypes();
    JoystickManager::registerQmlTypes();
#ifdef QGC_VIEWER3D
    Viewer3DManager::registerQmlTypes();
#endif

    qmlRegisterUncreatableType<GimbalController>("QGroundControl.Vehicle", 1, 0, "GimbalController", "Reference only");

#ifndef QGC_DISABLE_MAVLINK_INSPECTOR
    qmlRegisterUncreatableType<MAVLinkChartController>("QGroundControl", 1, 0, "MAVLinkChart", "Reference only");
    qmlRegisterType<MAVLinkInspectorController>("QGroundControl.Controllers", 1, 0, "MAVLinkInspectorController");
    qmlRegisterType<OverlayPhysics>("QGroundControl.Controls", 1, 0, "OverlayPhysics");
#endif
    qmlRegisterType<GeoTagController>("QGroundControl.Controllers", 1, 0, "GeoTagController");
    qmlRegisterType<LogDownloadController>("QGroundControl.Controllers", 1, 0, "LogDownloadController");
    qmlRegisterType<MAVLinkConsoleController>("QGroundControl.Controllers", 1, 0, "MAVLinkConsoleController");


    qmlRegisterUncreatableType<AutoPilotPlugin>("QGroundControl.AutoPilotPlugin", 1, 0, "AutoPilotPlugin", "Reference only");
    qmlRegisterType<ESP8266ComponentController>("QGroundControl.Controllers", 1, 0, "ESP8266ComponentController");
    qmlRegisterType<SyslinkComponentController>("QGroundControl.Controllers", 1, 0, "SyslinkComponentController");


    qmlRegisterUncreatableType<VehicleComponent>("QGroundControl.AutoPilotPlugin", 1, 0, "VehicleComponent", "Reference only");
#ifndef QGC_NO_SERIAL_LINK
    qmlRegisterType<FirmwareUpgradeController>("QGroundControl.Controllers", 1, 0, "FirmwareUpgradeController");
#endif
    qmlRegisterType<JoystickConfigController>("QGroundControl.Controllers", 1, 0, "JoystickConfigController");

    (void) qmlRegisterSingletonType<ShapeFileHelper>("QGroundControl.ShapeFileHelper", 1, 0, "ShapeFileHelper", [](QQmlEngine *, QJSEngine *) { return new ShapeFileHelper(); });

    qmlRegisterSingletonType<QGCMAVLink>("MAVLink", 1, 0, "MAVLink", mavlinkSingletonFactory);

    // Although this should really be in _initForNormalAppBoot putting it here allowws us to create unit tests which pop up more easily
    if(QFontDatabase::addApplicationFont(":/fonts/opensans") < 0) {
        qCWarning(QGCApplicationLog) << "Could not load /fonts/opensans font";
    }

    if(QFontDatabase::addApplicationFont(":/fonts/opensans-demibold") < 0) {
        qCWarning(QGCApplicationLog) << "Could not load /fonts/opensans-demibold font";
    }

    if (_simpleBootTest) {
        // Since GStream builds are so problematic we initialize video during the simple boot test
        // to make sure it works and verfies plugin availability.
        _initVideo();
    } else if (_runningUnitTests) {
        // SettingsManager is initialized above, so deep links can apply directly in tests.
        _settingsReady = true;
    } else {
        _initForNormalAppBoot();
    }
}

void QGCApplication::_initVideo()
{
#ifdef QGC_GST_STREAMING
    // Gstreamer video playback requires OpenGL
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
#endif

    QGCCorePlugin::instance();  // CorePlugin must be initialized before VideoManager for Video Cleanup
    VideoManager::instance();
    _videoManagerInitialized = true;
}

void QGCApplication::_initForNormalAppBoot()
{
    _initVideo(); // GStreamer must be initialized before QmlEngine

    QQuickStyle::setStyle("Basic");
    QGCCorePlugin::instance()->init();
    MAVLinkProtocol::instance()->init();
    MultiVehicleManager::instance()->init();
    _qmlAppEngine = QGCCorePlugin::instance()->createQmlApplicationEngine(this);
    QObject::connect(_qmlAppEngine, &QQmlApplicationEngine::objectCreationFailed, this, QCoreApplication::quit, Qt::QueuedConnection);
    QGCCorePlugin::instance()->createRootWindow(_qmlAppEngine);

    AudioOutput::instance()->init(SettingsManager::instance()->appSettings()->audioMuted());
    FollowMe::instance()->init();
    QGCPositionManager::instance()->init();
    LinkManager::instance()->init();
    VideoManager::instance()->init(mainRootWindow());
#ifdef QGC_WFB_ENABLED
    PacketRadioManager::instance()->init();
#endif
    DebugApiServer::startIfConfigured(this);

    // Settings and video are up: apply any aircast-qgc:// deep link that arrived
    // during launch (stored before this point), and allow later ones to apply live.
    _settingsReady = true;
    if (_pendingDeepLink.isValid()) {
        _applyDeepLink(_pendingDeepLink);
        _pendingDeepLink.clear();
    }

    // Image provider for Optical Flow
    _qmlAppEngine->addImageProvider(_qgcImageProviderId, new QGCImageProvider());

    // Safe to show popup error messages now that main window is created
    _showErrorsInToolbar = true;

    #ifdef Q_OS_LINUX
    #ifndef Q_OS_ANDROID
    #ifndef QGC_NO_SERIAL_LINK
        if (!_runningUnitTests) {
            // Determine if we have the correct permissions to access USB serial devices
            QFile permFile("/etc/group");
            if(permFile.open(QIODevice::ReadOnly)) {
                while(!permFile.atEnd()) {
                    const QString line = permFile.readLine();
                    if (line.contains("dialout") && !line.contains(getenv("USER"))) {
                        permFile.close();
                        showAppMessage(tr(
                            "The current user does not have the correct permissions to access serial devices. "
                            "You should also remove modemmanager since it also interferes.<br/><br/>"
                            "If you are using Ubuntu, execute the following commands to fix these issues:<br/>"
                            "<pre>sudo usermod -a -G dialout $USER<br/>"
                            "sudo apt-get remove modemmanager</pre>"));
                        break;
                    }
                }
                permFile.close();
            }
        }
    #endif
    #endif
    #endif

    // Now that main window is up check for lost log files
    MAVLinkProtocol::instance()->checkForLostLogFiles();

    // Load known link configurations
    LinkManager::instance()->loadLinkConfigurationList();

    // Probe for joysticks
    JoystickManager::instance()->init();

    if (_settingsUpgraded) {
        showAppMessage(tr("The format for %1 saved settings has been modified. "
                    "Your saved settings have been reset to defaults.").arg(applicationName()));
    }

    // Connect links with flag AutoconnectLink
    LinkManager::instance()->startAutoConnectedLinks();
}

void QGCApplication::deleteAllSettingsNextBoot()
{
    QSettings settings;
    settings.setValue(_deleteAllSettingsKey, true);
}

void QGCApplication::clearDeleteAllSettingsNextBoot()
{
    QSettings settings;
    settings.remove(_deleteAllSettingsKey);
}

void QGCApplication::reportMissingParameter(int componentId, const QString &name)
{
    const QPair<int, QString> missingParam(componentId, name);

    if (!_missingParams.contains(missingParam)) {
        _missingParams.append(missingParam);
    }
    _missingParamsDelayedDisplayTimer.start();
}

void QGCApplication::_missingParamsDisplay()
{
    if (_missingParams.isEmpty()) {
        return;
    }

    QString params;
    for (QPair<int, QString>& missingParam: _missingParams) {
        const QString param = QStringLiteral("%1:%2").arg(missingParam.first).arg(missingParam.second);
        if (params.isEmpty()) {
            params += param;
        } else {
            params += QStringLiteral(", %1").arg(param);
        }

    }
    _missingParams.clear();

    showAppMessage(tr("Parameters are missing from firmware. You may be running a version of firmware which is not fully supported or your firmware has a bug in it. Missing params: %1").arg(params));
}

QObject *QGCApplication::_rootQmlObject()
{
    if (_qmlAppEngine && _qmlAppEngine->rootObjects().size()) {
        return _qmlAppEngine->rootObjects()[0];
    }

    return nullptr;
}

void QGCApplication::showCriticalVehicleMessage(const QString &message)
{
    // PreArm messages are handled by Vehicle and shown in Map
    if (message.startsWith(QStringLiteral("PreArm")) || message.startsWith(QStringLiteral("preflight"), Qt::CaseInsensitive)) {
        return;
    }

    QObject *const rootQmlObject = _rootQmlObject();
    if (rootQmlObject && _showErrorsInToolbar) {
        QVariant varReturn;
        QVariant varMessage = QVariant::fromValue(message);
        QMetaObject::invokeMethod(rootQmlObject, "showCriticalVehicleMessage", Q_RETURN_ARG(QVariant, varReturn), Q_ARG(QVariant, varMessage));
    } else if (runningUnitTests() || !_showErrorsInToolbar) {
        // Unit tests can run without UI
        qCDebug(QGCApplicationLog) << "QGCApplication::showCriticalVehicleMessage unittest" << message;
    } else {
        qCWarning(QGCApplicationLog) << "Internal error";
    }
}

void QGCApplication::showAppMessage(const QString &message, const QString &title)
{
    const QString dialogTitle = title.isEmpty() ? applicationName() : title;

    QObject *const rootQmlObject = _rootQmlObject();
    if (rootQmlObject) {
        QVariant varReturn;
        QVariant varMessage = QVariant::fromValue(message);
        QMetaObject::invokeMethod(rootQmlObject, "_showMessageDialog", Q_RETURN_ARG(QVariant, varReturn), Q_ARG(QVariant, dialogTitle), Q_ARG(QVariant, varMessage));
    } else if (runningUnitTests()) {
        // Unit tests can run without UI
        qCDebug(QGCApplicationLog) << "QGCApplication::showAppMessage unittest title:message" << dialogTitle << message;
    } else {
        // UI isn't ready yet
        _delayedAppMessages.append(QPair<QString, QString>(dialogTitle, message));
        QTimer::singleShot(200, this, &QGCApplication::_showDelayedAppMessages);
    }
}

void QGCApplication::showRebootAppMessage(const QString &message, const QString &title)
{
    static QTime lastRebootMessage;

    const QTime currentTime = QTime::currentTime();
    const QTime previousTime = lastRebootMessage;
    lastRebootMessage = currentTime;

    if (previousTime.isValid() && (previousTime.msecsTo(currentTime) < (60 * 1000 * 2))) {
        // Debounce reboot messages
        return;
    }

    showAppMessage(message, title);
}

void QGCApplication::_showDelayedAppMessages()
{
    if (_rootQmlObject()) {
        for (const QPair<QString, QString>& appMsg: _delayedAppMessages) {
            showAppMessage(appMsg.second, appMsg.first);
        }
        _delayedAppMessages.clear();
    } else {
        QTimer::singleShot(200, this, &QGCApplication::_showDelayedAppMessages);
    }
}

QQuickWindow *QGCApplication::mainRootWindow()
{
    if (!_mainRootWindow) {
        _mainRootWindow = qobject_cast<QQuickWindow*>(_rootQmlObject());
    }

    return _mainRootWindow;
}

void QGCApplication::showVehicleConfig()
{
    if (_rootQmlObject()) {
      QMetaObject::invokeMethod(_rootQmlObject(), "showVehicleConfig");
    }
}

void QGCApplication::qmlAttemptWindowClose()
{
    if (_rootQmlObject()) {
        QMetaObject::invokeMethod(_rootQmlObject(), "attemptWindowClose");
    }
}

void QGCApplication::_checkForNewVersion()
{
    if (_runningUnitTests) {
        return;
    }

    if (!_parseVersionText(applicationVersion(), _majorVersion, _minorVersion, _buildVersion)) {
        return;
    }

    const QString versionCheckFile = QGCCorePlugin::instance()->stableVersionCheckFileUrl();
    if (!versionCheckFile.isEmpty()) {
        QGCFileDownload *const download = new QGCFileDownload(this);
        (void) connect(download, &QGCFileDownload::downloadComplete, this, &QGCApplication::_qgcCurrentStableVersionDownloadComplete);
        download->download(versionCheckFile);
    }
}

void QGCApplication::_qgcCurrentStableVersionDownloadComplete(const QString &remoteFile, const QString &localFile, const QString &errorMsg)
{
    Q_UNUSED(remoteFile);

    if (errorMsg.isEmpty()) {
        QFile versionFile(localFile);
        if (versionFile.open(QIODevice::ReadOnly)) {
            QTextStream textStream(&versionFile);
            const QString version = textStream.readLine();

            qCDebug(QGCApplicationLog) << version;

            int majorVersion, minorVersion, buildVersion;
            if (_parseVersionText(version, majorVersion, minorVersion, buildVersion)) {
                if (_majorVersion < majorVersion ||
                        ((_majorVersion == majorVersion) && (_minorVersion < minorVersion)) ||
                        ((_majorVersion == majorVersion) && (_minorVersion == minorVersion) && (_buildVersion < buildVersion))) {
                    showAppMessage(tr("There is a newer version of %1 available. You can download it from %2.").arg(applicationName()).arg(QGCCorePlugin::instance()->stableDownloadLocation()), tr("New Version Available"));
                }
            }
        }
    } else {
        qCDebug(QGCApplicationLog) << "Download QGC stable version failed" << errorMsg;
    }

    sender()->deleteLater();
}

bool QGCApplication::_parseVersionText(const QString &versionString, int &majorVersion, int &minorVersion, int &buildVersion)
{
    static const QRegularExpression regExp("v(\\d+)\\.(\\d+)\\.(\\d+)");
    const QRegularExpressionMatch match = regExp.match(versionString);
    if (match.hasMatch() && match.lastCapturedIndex() == 3) {
        majorVersion = match.captured(1).toInt();
        minorVersion = match.captured(2).toInt();
        buildVersion = match.captured(3).toInt();
        return true;
    }

    return false;
}

QString QGCApplication::cachedParameterMetaDataFile()
{
    QSettings settings;
    const QDir parameterDir = QFileInfo(settings.fileName()).dir();
    return parameterDir.filePath(QStringLiteral("ParameterFactMetaData.xml"));
}

QString QGCApplication::cachedAirframeMetaDataFile()
{
    QSettings settings;
    const QDir airframeDir = QFileInfo(settings.fileName()).dir();
    return airframeDir.filePath(QStringLiteral("PX4AirframeFactMetaData.xml"));
}

int QGCApplication::CompressedSignalList::_signalIndex(const QMetaMethod &method)
{
    if (method.methodType() != QMetaMethod::Signal) {
        qCWarning(QGCApplicationLog) << "Internal error:" << Q_FUNC_INFO <<  "not a signal" << method.methodType();
        return -1;
    }

    int index = -1;
    const QMetaObject *metaObject = method.enclosingMetaObject();
    for (int i=0; i<=method.methodIndex(); i++) {
        if (metaObject->method(i).methodType() != QMetaMethod::Signal) {
            continue;
        }
        index++;
    }

    return index;
}

void QGCApplication::CompressedSignalList::add(const QMetaMethod &method)
{
    const QMetaObject *metaObject = method.enclosingMetaObject();
    const int signalIndex = _signalIndex(method);

    if (signalIndex != -1 && !contains(metaObject, signalIndex)) {
        _signalMap[method.enclosingMetaObject()].insert(signalIndex);
    }
}

void QGCApplication::CompressedSignalList::remove(const QMetaMethod &method)
{
    const int signalIndex = _signalIndex(method);
    const QMetaObject *const metaObject = method.enclosingMetaObject();

    if (signalIndex != -1 && _signalMap.contains(metaObject) && _signalMap[metaObject].contains(signalIndex)) {
        _signalMap[metaObject].remove(signalIndex);
        if (_signalMap[metaObject].count() == 0) {
            _signalMap.remove(metaObject);
        }
    }
}

bool QGCApplication::CompressedSignalList::contains(const QMetaObject *metaObject, int signalIndex)
{
    return _signalMap.contains(metaObject) && _signalMap[metaObject].contains(signalIndex);
}

void QGCApplication::addCompressedSignal(const QMetaMethod &method)
{
    _compressedSignals.add(method);
}

void QGCApplication::removeCompressedSignal(const QMetaMethod &method)
{
    _compressedSignals.remove(method);
}

bool QGCApplication::compressEvent(QEvent *event, QObject *receiver, QPostEventList *postedEvents)
{
    if (event->type() != QEvent::MetaCall) {
        return QApplication::compressEvent(event, receiver, postedEvents);
    }

    const QMetaCallEvent *mce = static_cast<QMetaCallEvent*>(event);
    if (!mce->sender() || !_compressedSignals.contains(mce->sender()->metaObject(), mce->signalId())) {
        return QApplication::compressEvent(event, receiver, postedEvents);
    }

    for (QPostEventList::iterator it = postedEvents->begin(); it != postedEvents->end(); ++it) {
        QPostEvent &cur = *it;
        if (cur.receiver != receiver || cur.event == 0 || cur.event->type() != event->type()) {
            continue;
        }
        const QMetaCallEvent *cur_mce = static_cast<QMetaCallEvent*>(cur.event);
        if (cur_mce->sender() != mce->sender() || cur_mce->signalId() != mce->signalId() || cur_mce->id() != mce->id()) {
            continue;
        }
        /* Keep The Newest Call */
        // We can't merely qSwap the existing posted event with the new one, since QEvent
        // keeps track of whether it has been posted. Deletion of a formerly posted event
        // takes the posted event list mutex and does a useless search of the posted event
        // list upon deletion. We thus clear the QEvent::posted flag before deletion.
        struct EventHelper : private QEvent {
            static void clearPostedFlag(QEvent * ev) {
                (&static_cast<EventHelper*>(ev)->t)[1] &= ~0x8001; // Hack to clear QEvent::posted
            }
        };
        EventHelper::clearPostedFlag(cur.event);
        delete cur.event;
        cur.event = event;
        return true;
    }

    return false;
}

void QGCApplication::handleDeepLink(const QUrl &url)
{
    if (url.scheme() != QStringLiteral("aircast-qgc")) {
        return;
    }
    if (_settingsReady) {
        _applyDeepLink(url);
    } else {
        _pendingDeepLink = url;
    }
}

void QGCApplication::_applyDeepLink(const QUrl &url)
{
    const QUrlQuery query(url);
    const QString whep = query.queryItemValue(QStringLiteral("whep"), QUrl::FullyDecoded);
    const QString rtsp = query.queryItemValue(QStringLiteral("rtsp"), QUrl::FullyDecoded);
    const QString name = query.queryItemValue(QStringLiteral("name"), QUrl::FullyDecoded);
    const QString debug = query.queryItemValue(QStringLiteral("debug"), QUrl::FullyDecoded);

    if (!debug.isEmpty()) {
        bool ok = false;
        const uint port = debug.toUInt(&ok);
        if (ok && port > 0 && port <= 65535) {
            DebugApiServer::start(static_cast<quint16>(port), this);
            qCDebug(QGCApplicationLog) << "Enabled debug API via deep link on port" << port;
        } else {
            qCWarning(QGCApplicationLog) << "aircast-qgc deep link has invalid debug port" << debug;
        }
    }

    const QString deviceHost = query.queryItemValue(QStringLiteral("host"), QUrl::FullyDecoded);
    if (!deviceHost.isEmpty()) {
        _setupFromDevice(deviceHost);
        return;
    }

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    if (!videoSettings) {
        return;
    }

    if (!whep.isEmpty()) {
        videoSettings->whepUrl()->setRawValue(whep);
        videoSettings->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceWebRTC));
    } else if (!rtsp.isEmpty()) {
        videoSettings->rtspUrl()->setRawValue(rtsp);
        videoSettings->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceRTSP));
    } else {
        if (debug.isEmpty()) {
            qCWarning(QGCApplicationLog) << "aircast-qgc deep link has no whep/rtsp query" << url.toString();
        }
        return;
    }

    if (!name.isEmpty()) {
        videoSettings->primaryCameraName()->setRawValue(name);
    }

    qCDebug(QGCApplicationLog) << "Applied aircast-qgc deep link" << url.toString();
}

void QGCApplication::_setupFromDevice(const QString &host)
{
    // The web API may sit on a non-standard port (host:port), but cameras and
    // telemetry always live on the device's standard ports, so they use the bare host.
    const QString bareHost = host.section(QLatin1Char(':'), 0, 0);
    if (!_deviceSetupNetworkManager) {
        _deviceSetupNetworkManager = new QNetworkAccessManager(this);
    }
    QNetworkAccessManager *const nam = _deviceSetupNetworkManager;
    const int generation = ++_deviceSetupGeneration;
    const auto fetch = [this, nam, host, bareHost, generation](const QString &path, void (QGCApplication::*apply)(const QString&, const QJsonObject&)) {
        QNetworkReply *reply = nam->get(QNetworkRequest(QUrl(QStringLiteral("http://%1%2").arg(host, path))));
        connect(reply, &QNetworkReply::finished, this, [this, reply, host, bareHost, path, apply, generation]() {
            reply->deleteLater();
            if (generation != _deviceSetupGeneration) {
                // A newer _setupFromDevice() call has superseded this one; a late reply must not clobber it.
                return;
            }
            if (reply->error() != QNetworkReply::NoError) {
                qCWarning(QGCApplicationLog) << "Aircast device setup failed" << host << path << reply->errorString();
                return;
            }
            (this->*apply)(bareHost, QJsonDocument::fromJson(reply->readAll()).object());
        });
    };
    fetch(QStringLiteral("/api/stream/config"), &QGCApplication::_applyDeviceCameras);
    fetch(QStringLiteral("/api/telemetry/config"), &QGCApplication::_applyDeviceTelemetry);
}

void QGCApplication::_applyDeviceCameras(const QString &host, const QJsonObject &config)
{
    const QJsonObject paths = config.value(QStringLiteral("paths")).toObject();
    QStringList cams;
    for (auto it = paths.constBegin(); it != paths.constEnd(); ++it) {
        if (!it.value().toObject().value(QStringLiteral("source")).toString().isEmpty()) {
            cams.append(it.key());
        }
    }
    if (cams.isEmpty()) {
        qCWarning(QGCApplicationLog) << "Aircast device setup: no cameras configured on" << host;
        return;
    }

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    if (!videoSettings) {
        return;
    }
    videoSettings->rtspUrl()->setRawValue(QStringLiteral("rtsp://%1:8554/%2").arg(host, cams.first()));
    videoSettings->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceRTSP));
    videoSettings->primaryCameraName()->setRawValue(QStringLiteral("%1 (%2)").arg(cams.first(), host));

    QJsonArray extras;
    for (int i = 1; i < cams.size(); ++i) {
        extras.append(QJsonObject{
            {QStringLiteral("name"), QStringLiteral("%1 (%2)").arg(cams.at(i), host)},
            {QStringLiteral("source"), QString::fromUtf8(VideoSettings::videoSourceWebRTC)},
            {QStringLiteral("url"), QStringLiteral("http://%1:8889/%2/whep").arg(host, cams.at(i))},
        });
    }
    videoSettings->extraVideoSources()->setRawValue(QString::fromUtf8(QJsonDocument(extras).toJson(QJsonDocument::Compact)));

    qCDebug(QGCApplicationLog) << "Aircast device setup: configured" << cams.size() << "camera(s) from" << host;
}

void QGCApplication::_applyDeviceTelemetry(const QString &host, const QJsonObject &config)
{
    const QJsonArray endpoints = config.value(QStringLiteral("endpoints")).toArray();
    const auto serverPort = [&endpoints](const QString &scheme) -> quint16 {
        for (const QJsonValue &value : endpoints) {
            const QString spec = value.toString();
            if (!spec.startsWith(scheme + QLatin1Char(':'))) {
                continue;
            }
            const uint port = spec.section(QLatin1Char(':'), -1).toUInt();
            if (port > 0 && port <= 65535) {
                return static_cast<quint16>(port);
            }
        }
        return 0;
    };

    LinkManager *linkMgr = LinkManager::instance();
    const QString linkName = QStringLiteral("Aircast %1").arg(host);
    QmlObjectListModel *configs = linkMgr->linkConfigurations();
    for (int i = 0; i < configs->count(); ++i) {
        LinkConfiguration *existing = qobject_cast<LinkConfiguration*>(configs->get(i));
        if (existing && existing->name() == linkName) {
            linkMgr->removeConfiguration(existing);
            break;
        }
    }

    LinkConfiguration *linkConfig = nullptr;
    if (const quint16 udpPort = serverPort(QStringLiteral("udps"))) {
        UDPConfiguration *udpConfig = new UDPConfiguration(linkName);
        udpConfig->addHost(host, udpPort);
        linkConfig = udpConfig;
    } else if (const quint16 tcpPort = serverPort(QStringLiteral("tcps"))) {
        TCPConfiguration *tcpConfig = new TCPConfiguration(linkName);
        // TCPConfiguration stores a QHostAddress, so hostnames must be resolved here.
        QString address = host;
        if (QHostAddress(host).isNull()) {
            const QList<QHostAddress> resolved = QHostInfo::fromName(host).addresses();
            if (resolved.isEmpty()) {
                qCWarning(QGCApplicationLog) << "Aircast device setup: could not resolve" << host;
                delete tcpConfig;
                return;
            }
            address = resolved.first().toString();
        }
        tcpConfig->setHost(address);
        tcpConfig->setPort(tcpPort);
        linkConfig = tcpConfig;
    } else {
        qCWarning(QGCApplicationLog) << "Aircast device setup: no udps/tcps telemetry endpoint on" << host << endpoints;
        return;
    }

    linkConfig->setAutoConnect(true);
    SharedLinkConfigurationPtr sharedConfig = linkMgr->addConfiguration(linkConfig);
    linkMgr->saveLinkConfigurationList();
    if (linkMgr->createConnectedLink(sharedConfig)) {
        qCDebug(QGCApplicationLog) << "Aircast device setup: telemetry link connected" << linkName;
    } else {
        qCWarning(QGCApplicationLog) << "Aircast device setup: telemetry link failed to connect" << linkName;
    }
}

bool QGCApplication::event(QEvent *e)
{
    if (e->type() == QEvent::FileOpen) {
        // macOS delivers custom-scheme URLs (aircast-qgc://) as a file-open event.
        handleDeepLink(static_cast<QFileOpenEvent*>(e)->url());
        return true;
    }

    if (e->type() == QEvent::Quit && _mainRootWindow) {
        // On OSX if the user selects Quit from the menu (or Command-Q) the ApplicationWindow does not signal closing. Instead you get a Quit event here only.
        // This in turn causes the standard QGC shutdown sequence to not run. So in this case we close the window ourselves such that the
        // signal is sent and the normal shutdown sequence runs.
        const bool forceClose = _mainRootWindow->property("_forceClose").toBool();
        qCDebug(QGCApplicationLog) << "Quit event" << forceClose;
        // forceClose
        //  true:   Standard QGC shutdown sequence is complete. Let the app quit normally by falling through to the base class processing.
        //  false:  QGC shutdown sequence has not been run yet. Don't let this event close the app yet. Close the main window to kick off the normal shutdown.
        if (!forceClose) {
            //
            _mainRootWindow->close();
            e->ignore();
            return true;
        }
    }

    return QApplication::event(e);
}

QGCImageProvider *QGCApplication::qgcImageProvider()
{
    return dynamic_cast<QGCImageProvider*>(_qmlAppEngine->imageProvider(_qgcImageProviderId));
}

void QGCApplication::shutdown()
{
    qCDebug(QGCApplicationLog) << "Exit";

    if (_videoManagerInitialized) {
        VideoManager::instance()->cleanup();
    }

    QGCCorePlugin::instance()->cleanup();

    // This is bad, but currently qobject inheritances are incorrect and cause crashes on exit without
    delete _qmlAppEngine;
}

QString QGCApplication::numberToString(quint64 number)
{
    return getCurrentLanguage().toString(number);
}

QString QGCApplication::bigSizeToString(quint64 size)
{
    QString result;
    const QLocale kLocale = getCurrentLanguage();
    if (size < 1024) {
        result = kLocale.toString(size) + "B";
    } else if (size < pow(1024, 2)) {
        result = kLocale.toString(static_cast<double>(size) / 1024.0, 'f', 1) + "KB";
    } else if (size < pow(1024, 3)) {
        result = kLocale.toString(static_cast<double>(size) / pow(1024, 2), 'f', 1) + "MB";
    } else if (size < pow(1024, 4)) {
        result = kLocale.toString(static_cast<double>(size) / pow(1024, 3), 'f', 1) + "GB";
    } else {
        result = kLocale.toString(static_cast<double>(size) / pow(1024, 4), 'f', 1) + "TB";
    }
    return result;
}

QString QGCApplication::bigSizeMBToString(quint64 size_MB)
{
    QString result;
    const QLocale kLocale = getCurrentLanguage();
    if (size_MB < 1024) {
        result = kLocale.toString(static_cast<double>(size_MB) , 'f', 0) + " MB";
    } else if(size_MB < pow(1024, 2)) {
        result = kLocale.toString(static_cast<double>(size_MB) / 1024.0, 'f', 1) + " GB";
    } else {
        result = kLocale.toString(static_cast<double>(size_MB) / pow(1024, 2), 'f', 2) + " TB";
    }
    return result;
}
