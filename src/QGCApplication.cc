#include "QGCApplication.h"
#include "QGCHostNotices.h"

#include <QtCore/QEvent>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaMethod>
#include <QtCore/QMetaObject>
#include <QtCore/QRegularExpression>
#include <QtCore/private/qthread_p.h>
#include <QtCore/QUrlQuery>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#ifndef QGC_HEADLESS_CORE
#include <QtGui/QFileOpenEvent>
#include <QtGui/QFontDatabase>
#include <QtGui/QIcon>
#include <QtGui/QStyleHints>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
#include <QtQuick/QQuickImageProvider>
#include <QtQuick/QQuickWindow>
#include <QtQuickControls2/QQuickStyle>
#include <QtSvg/QSvgRenderer>

#include "PlatformTheme.h"
#endif

#include <QtCore/private/qthread_p.h>

#include "AppSettings.h"
#include "AudioOutput.h"
#include "DebugApiServer.h"
#include "FollowMe.h"
#include "JoystickManager.h"
#include "JsonParsing.h"
#include "LinkManager.h"
#include "LogManager.h"
#include "SkydroidH16Links.h"
#include "MAVLinkProtocol.h"
#include "MavlinkSettings.h"
#include "MultiVehicleManager.h"
#include "NTRIPManager.h"
#ifdef QGC_WFB_ENABLED
#include "PacketRadioManager.h"
#endif
#include "ParameterManager.h"
#include "PositionManager.h"
#include "QGCCommandLineParser.h"
#include "QGCCorePlugin.h"
#include "QGCFileDownload.h"
#include "QGCLoggingCategory.h"
#include "QGCLoggingCategoryManager.h"
#include "QGCNetworkHelper.h"
#include "QGCSettingsRecovery.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "VideoSettings.h"
#include "AircastAccount.h"
#include "AircastCloudLink.h"
#include "TCPLink.h"
#include "UDPLink.h"
#include "Vehicle.h"
#include "VideoManager.h"
#include "qgc_version.h"

#ifndef QGC_HEADLESS_CORE
#include "ColoredSvgImageProvider.h"
#include "GraphicsSetup.h"
#include "OverlayPhysics.h"
#include "QGCImageProvider.h"
#endif
#ifndef QGC_NO_SERIAL_LINK
#include "SerialLink.h"
#endif

QGC_LOGGING_CATEGORY(QGCApplicationLog, "API.QGCApplication")
QGC_LOGGING_CATEGORY(QGCAppMessageLog, "API.QGCApplication.AppMessage")

QGCApplication::QGCApplication(int& argc, char* argv[], const QGCCommandLineParser::CommandLineParseResult& cli, bool embeddedHost)
    : QGC_APPLICATION_BASE(argc, argv),
      _runningUnitTests(cli.runningUnitTests),
      _simpleBootTest(cli.simpleBootTest),
      _fakeMobile(cli.fakeMobile),
      _logOutput(cli.logOutput),
      _embeddedHost(embeddedHost),
      _systemId(cli.systemId.value_or(0))
{
    _msecsElapsedTime.start();

    QGCNetworkHelper::initializeProxySupport();

    bool fClearSettingsOptions = cli.clearSettingsOptions;
    const bool fClearCache = cli.clearCache;
    const QString loggingOptions = cli.loggingOptions.value_or(QString(""));

    _missingParamsDelayedDisplayTimer.setSingleShot(true);
    _missingParamsDelayedDisplayTimer.setInterval(_missingParamsDelayedDisplayTimerTimeout);
    (void) connect(&_missingParamsDelayedDisplayTimer, &QTimer::timeout, this, &QGCApplication::_missingParamsDisplay);

    QString applicationName;
    if (_runningUnitTests || _simpleBootTest) {
        if (!cli.unitTests.isEmpty()) {
            applicationName = QStringLiteral("%1_unittest_%2").arg(QGC_APP_NAME, cli.unitTests.first());
        } else {
            applicationName =
                QStringLiteral("%1_unittest_%2").arg(QGC_APP_NAME).arg(QCoreApplication::applicationPid());
        }
    } else {
#ifdef QGC_DAILY_BUILD
        applicationName = QStringLiteral("%1 Daily").arg(QGC_APP_NAME);
#else
        applicationName = QGC_APP_NAME;
#endif
    }
    setApplicationName(applicationName);
#ifndef QGC_HEADLESS_CORE
    setDesktopFileName(QGC_PACKAGE_NAME);
#endif
    setOrganizationName(QGC_ORG_NAME);
    setOrganizationDomain(QGC_ORG_DOMAIN);
    setApplicationVersion(QString(QGC_APP_VERSION_STR));
#ifndef QGC_HEADLESS_CORE
    styleHints()->setMousePressAndHoldInterval(500);
#ifdef Q_OS_LINUX
    setWindowIcon(QIcon(":/res/qgroundcontrol.ico"));
#endif
#endif

    QSettings::setDefaultFormat(QSettings::IniFormat);
    QSettings settings;
    qCDebug(QGCApplicationLog) << "Settings location" << settings.fileName()
                               << "Is writable?:" << settings.isWritable();

    if (QGCSettingsRecovery::moveAsideIfUnwritable(settings)) {
        qCDebug(QGCApplicationLog) << "Settings location recovered, Is writable?:" << settings.isWritable();
    } else if (!settings.isWritable()) {
        qCWarning(QGCApplicationLog) << "Settings location is not writable";
    }

    fClearSettingsOptions |= settings.value(AppSettings::clearSettingsNextBootKey, false).toBool();

    if (_runningUnitTests || _simpleBootTest) {
        fClearSettingsOptions = true;
    }

    if (fClearSettingsOptions) {
        settings.clear();

        QDir paramDir(ParameterManager::parameterCacheDir());
        paramDir.removeRecursively();
        paramDir.mkpath(paramDir.absolutePath());
    } else {
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
        QFile parameter(cachedParameterMetaDataFile());
        parameter.remove();
        QFile airframe(cachedAirframeMetaDataFile());
        airframe.remove();

        const QString metaDataCachePath =
            QStandardPaths::writableLocation(QStandardPaths::CacheLocation) + QStringLiteral("/ParameterMetaData");
        QDir(metaDataCachePath).removeRecursively();
    }

    QGCLoggingCategoryManager::init();
    QGCLoggingCategoryManager::instance()->installFilter(loggingOptions);

    if (_runningUnitTests) {
        QGCLoggingCategoryManager::instance()->setCategoryEnabled(QStringLiteral("API.QGCApplication.AppMessage"),
                                                                  true);
    }

    setLanguage();

#ifndef QGC_HEADLESS_CORE
    QSvgRenderer::setDefaultOptions(QtSvg::Tiny12FeaturesOnly);
#endif

#ifndef QGC_DAILY_BUILD
    _checkForNewVersion();
#endif
}

void QGCApplication::setLanguage()
{
    _locale = QLocale::system();
    qCDebug(QGCApplicationLog) << "System reported locale:" << _locale << "; Name" << _locale.name()
                               << "; Preffered (used in maps): "
                               << (QLocale::system().uiLanguages().length() > 0 ? QLocale::system().uiLanguages()[0]
                                                                                : "None");

    QLocale::Language possibleLocale = AppSettings::_qLocaleLanguageEarlyAccess();
    if (possibleLocale != QLocale::AnyLanguage) {
        _locale = QLocale(possibleLocale);
    }
    if (_locale == QLocale::Korean) {
        qCDebug(QGCApplicationLog) << "Loading Korean fonts" << _locale.name();
#ifndef QGC_HEADLESS_CORE
        if (QFontDatabase::addApplicationFont(":/fonts/NanumGothic-Regular") < 0) {
            qCWarning(QGCApplicationLog) << "Could not load /fonts/NanumGothic-Regular font";
        }
        if (QFontDatabase::addApplicationFont(":/fonts/NanumGothic-Bold") < 0) {
            qCWarning(QGCApplicationLog) << "Could not load /fonts/NanumGothic-Bold font";
        }
#endif
    }
    qCDebug(QGCApplicationLog) << "Loading localizations for" << _locale.name();
    removeTranslator(JsonParsing::translator());
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
        if (JsonParsing::translator()->load(_locale, QLatin1String("qgc_json_"), "", ":/i18n")) {
            installTranslator(JsonParsing::translator());
        } else {
            qCWarning(QGCApplicationLog) << "Error loading json localization for" << _locale.name();
        }
    }

#ifndef QGC_HEADLESS_CORE
    if (_qmlAppEngine) {
        _qmlAppEngine->retranslate();
    }
#endif

    emit languageChanged(_locale);
}

QGCApplication::~QGCApplication() {}

void QGCApplication::init()
{
    SettingsManager::instance()->init();
    if (_systemId > 0) {
        qCDebug(QGCApplicationLog) << "Setting MAVLink System ID to:" << _systemId;
        SettingsManager::instance()->mavlinkSettings()->gcsMavlinkSystemID()->setRawValue(_systemId);
    }

    LogManager::instance()->init();

#ifndef QGC_HEADLESS_CORE
    qmlRegisterType<OverlayPhysics>("QGroundControl.Controls", 1, 0, "OverlayPhysics");

    if (QFontDatabase::addApplicationFont(":/fonts/opensans") < 0) {
        qCWarning(QGCApplicationLog) << "Could not load /fonts/opensans font";
    }

    if (QFontDatabase::addApplicationFont(":/fonts/opensans-demibold") < 0) {
        qCWarning(QGCApplicationLog) << "Could not load /fonts/opensans-demibold font";
    }
#endif

    if (_simpleBootTest) {
        const bool videoInitialized = _initVideo();
#ifdef QGC_HEADLESS_CORE
        QGCCorePlugin::instance()->init();
        MAVLinkProtocol::instance()->init();
        MultiVehicleManager::instance()->init();
        const bool qmlRootLoaded = true;
#else
        const bool qmlRootLoaded = _initQmlRootWindow();
#endif
        _bootTestPassed = videoInitialized && qmlRootLoaded;
    } else if (_runningUnitTests) {
        _settingsReady = true;
    } else {
        _initForNormalAppBoot();
    }
}

bool QGCApplication::_initVideo()
{
#ifdef QGC_GST_STREAMING
    qCDebug(QGCApplicationLog) << "Using default graphics API for appsink → VideoOutput video path";
#endif

    QGCCorePlugin::instance();
    VideoManager* videoManager = VideoManager::instance();
    videoManager->startVideoBackendInit();
    const bool initSucceeded = !_simpleBootTest || videoManager->waitForVideoBackendReady();
    _videoManagerInitialized = true;
    return initSucceeded;
}

#ifndef QGC_HEADLESS_CORE
bool QGCApplication::_initQmlRootWindow()
{
    QQuickStyle::setStyle(PlatformTheme::instance()->controlStyle());
    QGCCorePlugin::instance()->init();
    MAVLinkProtocol::instance()->init();
    MultiVehicleManager::instance()->init();

    if (_embeddedHost) {
        return true;
    }

    _qmlAppEngine = QGCCorePlugin::instance()->createQmlApplicationEngine(this);
    QObject::connect(_qmlAppEngine, &QQmlApplicationEngine::objectCreationFailed, this, QCoreApplication::quit,
                     Qt::QueuedConnection);

    _qmlAppEngine->addImageProvider(_qgcImageProviderId, new QGCImageProvider());
    _qmlAppEngine->addImageProvider(QLatin1String(ColoredSvgImageProvider::ProviderId), new ColoredSvgImageProvider());

    QGCCorePlugin::instance()->createRootWindow(_qmlAppEngine);

    GraphicsSetup::configureMainWindow(mainRootWindow());

    return mainRootWindow() != nullptr;
}

#endif

void QGCApplication::_initForNormalAppBoot()
{
    (void) _initVideo();

#ifdef QGC_HEADLESS_CORE
    QGCCorePlugin::instance()->init();
    MAVLinkProtocol::instance()->init();
    MultiVehicleManager::instance()->init();
#else
    (void) _initQmlRootWindow();
#endif

    AudioOutput::instance()->init(SettingsManager::instance()->appSettings()->audioVolume(),
                                  SettingsManager::instance()->appSettings()->audioMuted());
    FollowMe::instance()->init();
    QGCPositionManager::instance()->init();
    NTRIPManager::instance()->init();
    LinkManager::instance()->init();
#ifdef QGC_HEADLESS_CORE
    VideoManager::instance()->init();
#else
    if (!_embeddedHost) {
        VideoManager::instance()->init(mainRootWindow());
    }
#endif
#ifdef QGC_WFB_ENABLED
    PacketRadioManager::instance()->init();
#endif
    DebugApiServer::startIfConfigured(this);

    _settingsReady = true;
    if (_pendingDeepLink.isValid()) {
        _applyDeepLink(_pendingDeepLink);
        _pendingDeepLink.clear();
    }

#if defined(Q_OS_LINUX) && !defined(QGC_HEADLESS_CORE)
    if (_qmlAppEngine) {
        QUrl windowIcon = QUrl("qrc:/res/qgroundcontrol.ico");
        windowIcon = _qmlAppEngine->interceptUrl(windowIcon, QQmlAbstractUrlInterceptor::UrlString);
        setWindowIcon(QIcon(":" + windowIcon.path()));
    }
#endif

    _showErrorsInToolbar = true;

#ifdef Q_OS_LINUX
#ifndef Q_OS_ANDROID
#ifndef QGC_NO_SERIAL_LINK
    if (!_runningUnitTests) {
        QFile permFile("/etc/group");
        if (permFile.open(QIODevice::ReadOnly)) {
            while (!permFile.atEnd()) {
                const QString line = permFile.readLine();
                if (line.contains("dialout") && !line.contains(getenv("USER"))) {
                    permFile.close();
                    showAppMessage(
                        tr("The current user does not have the correct permissions to access serial devices. "
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

    MAVLinkProtocol::instance()->checkForLostLogFiles();

    LinkManager::instance()->loadLinkConfigurationList();
    if (SkydroidH16Links::isThisRemote()) {
        SkydroidH16Links::ensure(LinkManager::instance(), SettingsManager::instance()->autoConnectSettings(), SettingsManager::instance()->videoSettings());
    }

    JoystickManager::instance()->init();

    if (_settingsUpgraded) {
        showAppMessage(tr("The format for %1 saved settings has been modified. "
                          "Your saved settings have been reset to defaults.")
                           .arg(applicationName()));
    }

    LinkManager::instance()->startAutoConnectedLinks();
}

void QGCApplication::reportMissingParameter(int componentId, const QString& name)
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
    for (QPair<int, QString>& missingParam : _missingParams) {
        const QString param = QStringLiteral("%1:%2").arg(missingParam.first).arg(missingParam.second);
        if (params.isEmpty()) {
            params += param;
        } else {
            params += QStringLiteral(", %1").arg(param);
        }
    }
    _missingParams.clear();

    showAppMessage(tr("Parameters are missing from firmware. You may be running a version of firmware which is not "
                      "fully supported or your firmware has a bug in it. Missing params: %1")
                       .arg(params));
}

QObject* QGCApplication::_rootQmlObject()
{
#ifndef QGC_HEADLESS_CORE
    if (_qmlAppEngine && _qmlAppEngine->rootObjects().size()) {
        return _qmlAppEngine->rootObjects()[0];
    }
#endif

    return nullptr;
}

void QGCApplication::showCriticalVehicleMessage(const QString& message)
{
    if (message.startsWith(QStringLiteral("PreArm")) ||
        message.startsWith(QStringLiteral("preflight"), Qt::CaseInsensitive)) {
        return;
    }

    QGCHostNotices::instance()->post(QGCHostNotices::VehicleError, applicationName(), message);

    QObject *const rootQmlObject = _rootQmlObject();
    if (rootQmlObject && _showErrorsInToolbar) {
        QVariant varReturn;
        QVariant varMessage = QVariant::fromValue(message);
        QMetaObject::invokeMethod(rootQmlObject, "showCriticalVehicleMessage", Q_RETURN_ARG(QVariant, varReturn),
                                  Q_ARG(QVariant, varMessage));
    } else if (runningUnitTests() || !_showErrorsInToolbar) {
        qCDebug(QGCApplicationLog) << "QGCApplication::showCriticalVehicleMessage unittest" << message;
    } else {
        qCWarning(QGCApplicationLog) << "Internal error";
    }
}

void QGCApplication::showAppMessage(const QString& message, const QString& title)
{
    const QString dialogTitle = title.isEmpty() ? applicationName() : title;
    QGCHostNotices::instance()->post(QGCHostNotices::Message, dialogTitle, message);

    if (runningUnitTests()) {
        qCDebug(QGCAppMessageLog) << "showAppMessage:" << dialogTitle << "-" << message;
        if (!_uiTestMode) {
            return;
        }
    }

    QObject* const rootQmlObject = _rootQmlObject();
    if (rootQmlObject) {
        QVariant varReturn;
        QVariant varMessage = QVariant::fromValue(message);
        QMetaObject::invokeMethod(rootQmlObject, "_showMessageDialog", Q_RETURN_ARG(QVariant, varReturn),
                                  Q_ARG(QVariant, dialogTitle), Q_ARG(QVariant, varMessage));
    } else {
        if (!_embeddedHost) {
            _delayedAppMessages.append(QPair<QString, QString>(dialogTitle, message));
            QTimer::singleShot(200, this, &QGCApplication::_showDelayedAppMessages);
        }
    }
}

bool QGCApplication::_rebootMessageDebounced()
{
    const QTime currentTime = QTime::currentTime();
    const QTime previousTime = _lastRebootMessageTime;
    _lastRebootMessageTime = currentTime;

    return previousTime.isValid() && (previousTime.msecsTo(currentTime) < (60 * 1000 * 2));
}

void QGCApplication::showRebootAppMessage(const QString& message, const QString& title)
{
    if (_rebootMessageDebounced()) {
        return;
    }

    showAppMessage(message, title);
}

void QGCApplication::showRebootVehicleMessage(const QString& message, const QString& title)
{
    if (_rebootMessageDebounced()) {
        return;
    }

    const QString dialogTitle = title.isEmpty() ? applicationName() : title;

    if (runningUnitTests()) {
        qCDebug(QGCAppMessageLog) << "showAppMessage:" << dialogTitle << "-" << message;
        if (!_uiTestMode) {
            return;
        }
    }

    QObject* const rootQmlObject = _rootQmlObject();
    if (rootQmlObject) {
        QVariant varReturn;
        QVariant varMessage = QVariant::fromValue(message);
        QMetaObject::invokeMethod(rootQmlObject, "_showRebootVehicleDialog", Q_RETURN_ARG(QVariant, varReturn),
                                  Q_ARG(QVariant, dialogTitle), Q_ARG(QVariant, varMessage));
    } else {
        _delayedAppMessages.append(QPair<QString, QString>(dialogTitle, message));
        QTimer::singleShot(200, this, &QGCApplication::_showDelayedAppMessages);
    }
}

void QGCApplication::_showDelayedAppMessages()
{
    if (_rootQmlObject()) {
        for (const QPair<QString, QString>& appMsg : _delayedAppMessages) {
            showAppMessage(appMsg.second, appMsg.first);
        }
        _delayedAppMessages.clear();
    } else if (!_embeddedHost) {
        QTimer::singleShot(200, this, &QGCApplication::_showDelayedAppMessages);
    }
}

#ifndef QGC_HEADLESS_CORE
QQuickWindow* QGCApplication::mainRootWindow()
{
    if (!_mainRootWindow) {
        _mainRootWindow = qobject_cast<QQuickWindow*>(_rootQmlObject());
    }

    return _mainRootWindow;
}
#endif

void QGCApplication::showVehicleConfig()
{
    QGCHostNotices::instance()->post(QGCHostNotices::Navigation, QStringLiteral("setup"), QString());
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
        QGCFileDownload* const download = new QGCFileDownload(this);
        (void) connect(download, &QGCFileDownload::finished, this,
                       &QGCApplication::_qgcCurrentStableVersionDownloadComplete);
        if (!download->start(versionCheckFile)) {
            qCDebug(QGCApplicationLog) << "Download QGC stable version failed to start" << download->errorString();
            download->deleteLater();
        }
    }
}

void QGCApplication::_qgcCurrentStableVersionDownloadComplete(bool success, const QString& localFile,
                                                              const QString& errorMsg)
{
    if (success) {
        QFile versionFile(localFile);
        if (versionFile.open(QIODevice::ReadOnly)) {
            QTextStream textStream(&versionFile);
            const QString version = textStream.readLine();

            qCDebug(QGCApplicationLog) << version;

            int majorVersion, minorVersion, buildVersion;
            if (_parseVersionText(version, majorVersion, minorVersion, buildVersion)) {
                if (_majorVersion < majorVersion ||
                    ((_majorVersion == majorVersion) && (_minorVersion < minorVersion)) ||
                    ((_majorVersion == majorVersion) && (_minorVersion == minorVersion) &&
                     (_buildVersion < buildVersion))) {
                    showAppMessage(tr("There is a newer version of %1 available. You can download it from %2.")
                                       .arg(applicationName())
                                       .arg(QGCCorePlugin::instance()->stableDownloadLocation()),
                                   tr("New Version Available"));
                }
            }
        }
    } else if (!errorMsg.isEmpty()) {
        qCDebug(QGCApplicationLog) << "Download QGC stable version failed" << errorMsg;
    }

    sender()->deleteLater();
}

bool QGCApplication::_parseVersionText(const QString& versionString, int& majorVersion, int& minorVersion,
                                       int& buildVersion)
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
    return parameterDir.filePath(QStringLiteral("ParameterFactMetaData.json"));
}

QString QGCApplication::cachedAirframeMetaDataFile()
{
    QSettings settings;
    const QDir airframeDir = QFileInfo(settings.fileName()).dir();
    return airframeDir.filePath(QStringLiteral("PX4AirframeFactMetaData.xml"));
}

int QGCApplication::CompressedSignalList::_signalIndex(const QMetaMethod& method)
{
    if (method.methodType() != QMetaMethod::Signal) {
        qCWarning(QGCApplicationLog) << "Internal error:" << Q_FUNC_INFO << "not a signal" << method.methodType();
        return -1;
    }

    int index = -1;
    const QMetaObject* metaObject = method.enclosingMetaObject();
    for (int i = 0; i <= method.methodIndex(); i++) {
        if (metaObject->method(i).methodType() != QMetaMethod::Signal) {
            continue;
        }
        index++;
    }

    return index;
}

void QGCApplication::CompressedSignalList::add(const QMetaMethod& method)
{
    const QMetaObject* metaObject = method.enclosingMetaObject();
    const int signalIndex = _signalIndex(method);

    if (signalIndex != -1 && !contains(metaObject, signalIndex)) {
        _signalMap[method.enclosingMetaObject()].insert(signalIndex);
    }
}

void QGCApplication::CompressedSignalList::remove(const QMetaMethod& method)
{
    const int signalIndex = _signalIndex(method);
    const QMetaObject* const metaObject = method.enclosingMetaObject();

    if (signalIndex != -1 && _signalMap.contains(metaObject) && _signalMap[metaObject].contains(signalIndex)) {
        _signalMap[metaObject].remove(signalIndex);
        if (_signalMap[metaObject].count() == 0) {
            _signalMap.remove(metaObject);
        }
    }
}

bool QGCApplication::CompressedSignalList::contains(const QMetaObject* metaObject, int signalIndex)
{
    return _signalMap.contains(metaObject) && _signalMap[metaObject].contains(signalIndex);
}

void QGCApplication::addCompressedSignal(const QMetaMethod& method)
{
    _compressedSignals.add(method);
}

void QGCApplication::removeCompressedSignal(const QMetaMethod& method)
{
    _compressedSignals.remove(method);
}

QT_WARNING_PUSH

QT_WARNING_DISABLE_DEPRECATED
bool QGCApplication::compressEvent(QEvent* event, QObject* receiver, QPostEventList* postedEvents)
{
    if (event->type() != QEvent::MetaCall) {
        return QGC_APPLICATION_BASE::compressEvent(event, receiver, postedEvents);
    }

    const QMetaCallEvent* mce = static_cast<QMetaCallEvent*>(event);
    if (!mce->sender() || !_compressedSignals.contains(mce->sender()->metaObject(), mce->signalId())) {
        return QGC_APPLICATION_BASE::compressEvent(event, receiver, postedEvents);
    }

    struct MetaCallHelper : public QMetaCallEvent {
        int id() const { return d.method_offset_ + d.method_relative_; }
    };
    const auto methodId = [](const QMetaCallEvent *e) { return static_cast<const MetaCallHelper*>(e)->id(); };

    for (QPostEventList::iterator it = postedEvents->begin(); it != postedEvents->end(); ++it) {
        QPostEvent& cur = *it;
        if (cur.receiver != receiver || cur.event == 0 || cur.event->type() != event->type()) {
            continue;
        }
        const QMetaCallEvent* cur_mce = static_cast<QMetaCallEvent*>(cur.event);
        if (cur_mce->sender() != mce->sender() || cur_mce->signalId() != mce->signalId() ||
            methodId(cur_mce) != methodId(mce)) {
            continue;
        }

        struct EventHelper : private QEvent
        {
            static void clearPostedFlag(QEvent* ev)
            {
                (&static_cast<EventHelper*>(ev)->t)[1] &= ~0x8001;
            }
        };

        EventHelper::clearPostedFlag(cur.event);
        delete cur.event;
        cur.event = event;
        return true;
    }

    return false;
}

QT_WARNING_POP

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
        if (!ok || port == 0 || port > 65535) {
            qCWarning(QGCApplicationLog) << "aircast-qgc deep link has invalid debug port" << debug;
        } else if (DebugApiServer::start(static_cast<quint16>(port), this)) {
            qCDebug(QGCApplicationLog) << "Enabled debug API via deep link on port" << port;
        } else {
            qCWarning(QGCApplicationLog) << "aircast-qgc deep link asked for the debug API, which this build does not include";
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

void QGCApplication::_removeLinkConfigurationNamed(const QString &name)
{
    LinkManager *linkMgr = LinkManager::instance();
    QmlObjectListModel *configs = linkMgr->linkConfigurations();
    for (int i = 0; i < configs->count(); ++i) {
        LinkConfiguration *existing = qobject_cast<LinkConfiguration*>(configs->get(i));
        if (existing && existing->name() == name) {
            linkMgr->removeConfiguration(existing);
            return;
        }
    }
}

void QGCApplication::_applyDeviceCloud(const QString &host, const QJsonObject &cloud)
{
    const QString apiBase = cloud.value(QStringLiteral("api")).toString();
    const QString deviceId = cloud.value(QStringLiteral("deviceId")).toString();
    if (apiBase.isEmpty() || deviceId.isEmpty()) {
        return;
    }

    AircastAccount::instance()->setApiBase(apiBase);

    const QString linkName = QStringLiteral("Aircast %1 (cloud)").arg(host);
    _removeLinkConfigurationNamed(linkName);
    AircastCloudConfiguration *cloudConfig = new AircastCloudConfiguration(linkName);
    cloudConfig->setApiBase(apiBase);
    cloudConfig->setDeviceId(deviceId);
    cloudConfig->setAutoConnect(true);

    LinkManager *linkMgr = LinkManager::instance();
    SharedLinkConfigurationPtr sharedConfig = linkMgr->addConfiguration(cloudConfig);
    linkMgr->saveLinkConfigurationList();
    if (!linkMgr->createConnectedLink(sharedConfig)) {
        qCWarning(QGCApplicationLog) << "Aircast device setup: cloud link failed to start" << linkName;
    }
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

    _applyDeviceCloud(host, config.value(QStringLiteral("cloud")).toObject());

    LinkManager *linkMgr = LinkManager::instance();
    const QString linkName = QStringLiteral("Aircast %1").arg(host);
    _removeLinkConfigurationNamed(linkName);

    LinkConfiguration *linkConfig = nullptr;
    if (const quint16 udpPort = serverPort(QStringLiteral("udps"))) {
        UDPConfiguration *udpConfig = new UDPConfiguration(linkName);
        udpConfig->addHost(host, udpPort);
        linkConfig = udpConfig;
    } else if (const quint16 tcpPort = serverPort(QStringLiteral("tcps"))) {
        TCPConfiguration *tcpConfig = new TCPConfiguration(linkName);
        tcpConfig->setHost(host);
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

void QGCApplication::closeVehicleConnections()
{
    if (_connectionsClosed) {
        return;
    }
    _connectionsClosed = true;

    LinkManager::instance()->shutdown();
    if (_videoManagerInitialized) {
        VideoManager::instance()->stopVideo();
    }
}

bool QGCApplication::event(QEvent* e)
{
#ifndef QGC_HEADLESS_CORE
    if (e->type() == QEvent::FileOpen) {
        handleDeepLink(static_cast<QFileOpenEvent*>(e)->url());
        return true;
    }
#endif

    if (e->type() == QEvent::Quit) {
#ifdef QGC_HEADLESS_CORE
        closeVehicleConnections();
        return QGC_APPLICATION_BASE::event(e);
#else
        if (!_mainRootWindow) {
            closeVehicleConnections();
            return QGC_APPLICATION_BASE::event(e);
        }
        const bool forceClose = _mainRootWindow->property("_forceClose").toBool();
        qCDebug(QGCApplicationLog) << "Quit event" << forceClose;
        if (!forceClose) {
            _mainRootWindow->close();
            e->ignore();
            return true;
        }
#endif
    }

    return QGC_APPLICATION_BASE::event(e);
}

#ifndef QGC_HEADLESS_CORE
QGCImageProvider* QGCApplication::qgcImageProvider()
{
    if (!_qmlAppEngine) {
        return nullptr;
    }

    return dynamic_cast<QGCImageProvider*>(_qmlAppEngine->imageProvider(_qgcImageProviderId));
}
#endif

void QGCApplication::shutdown()
{
    qCDebug(QGCApplicationLog) << "Exit";

    if (_videoManagerInitialized) {
        VideoManager::instance()->cleanup();
    }

#ifndef QGC_HEADLESS_CORE
    if (_qmlAppEngine) {
        QGCCorePlugin::instance()->destroyQmlApplicationEngine(_qmlAppEngine);
        _qmlAppEngine = nullptr;
    }
#endif

    QGCCorePlugin::instance()->cleanup();

    if (_runningUnitTests || _simpleBootTest) {
        const QSettings settings;
        const QString settingsFile = settings.fileName();
        if (QFile::exists(settingsFile)) {
            if (QFile::remove(settingsFile)) {
                qCDebug(QGCApplicationLog) << "Removed test run settings file:" << settingsFile;
            } else {
                qCWarning(QGCApplicationLog) << "Failed to remove test run settings file:" << settingsFile;
            }
        }

        QDir settingsAppDir(ParameterManager::parameterCacheDir());
        settingsAppDir.cdUp();
        if (settingsAppDir.exists()) {
            if (settingsAppDir.removeRecursively()) {
                qCDebug(QGCApplicationLog) << "Removed test run settings directory:" << settingsAppDir.absolutePath();
            } else {
                qCWarning(QGCApplicationLog)
                    << "Failed to remove test run settings directory:" << settingsAppDir.absolutePath();
            }
        }

        QDir appDir(SettingsManager::instance()->appSettings()->savePath()->rawValue().toString());
        if (appDir.exists()) {
            if (appDir.removeRecursively()) {
                qCDebug(QGCApplicationLog) << "Removed test run app data directory:" << appDir.absolutePath();
            } else {
                qCWarning(QGCApplicationLog)
                    << "Failed to remove test run app data directory:" << appDir.absolutePath();
            }
        }
    }

#ifndef QGC_HEADLESS_CORE
    delete _qmlAppEngine;
#endif
}
