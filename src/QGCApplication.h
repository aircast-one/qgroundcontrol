#pragma once

#include <QtCore/QElapsedTimer>
#include <QtCore/QLoggingCategory>
#include <QtCore/QMap>
#include <QtCore/QSet>
#include <QtCore/QTime>
#include <QtCore/QTimer>
#include <QtCore/QTranslator>
#include <QtGui/QGuiApplication>
#include <QtCore/QUrl>

namespace QGCCommandLineParser {
    struct CommandLineParseResult;
}

class QQmlApplicationEngine;
class QQuickWindow;
class QGCImageProvider;
class QGCApplication;
class QEvent;
class QJsonObject;
class QPostEventList;
class QMetaMethod;
struct QMetaObject;
class QNetworkAccessManager;

#if defined(qApp)
#undef qApp
#endif
#define qApp (static_cast<QGCApplication*>(QGuiApplication::instance()))

#if defined(qGuiApp)
#undef qGuiApp
#endif
#define qGuiApp (static_cast<QGCApplication*>(QGuiApplication::instance()))

#define qgcApp() qApp

/// \brief The main application and management class.
///
class QGCApplication : public QGuiApplication
{
    Q_OBJECT

    /// Unit Test have access to creating and destroying singletons
    friend class UnitTest;
public:
    QGCApplication(int &argc, char *argv[], const QGCCommandLineParser::CommandLineParseResult &args, bool embeddedHost = false);
    ~QGCApplication();

    bool runningUnitTests() const { return _runningUnitTests; }
    bool simpleBootTest() const { return _simpleBootTest; }
    bool embeddedHost() const { return _embeddedHost; }
    void showVehicleConfig();
    bool bootTestPassed() const { return _bootTestPassed; }

    /// Returns true if Qt debug output should be logged to a file
    bool logOutput() const { return _logOutput; }

    /// Used to report a missing Parameter. Warning will be displayed to user. Method may be called
    /// multiple times.
    void reportMissingParameter(int componentId, const QString &name);

    /// @return true: Fake ui into showing mobile interface
    bool fakeMobile() const { return _fakeMobile; }

    void setLanguage();
    QQuickWindow *mainRootWindow();
    uint64_t msecsSinceBoot() const { return _msecsElapsedTime.elapsed(); }

    /// Registers the signal such that only the last duplicate signal added is left in the queue.
    void addCompressedSignal(const QMetaMethod &method);

    void removeCompressedSignal(const QMetaMethod &method);

    bool event(QEvent *e) final;

    /// Shut the link manager and video down, the same sequence QML's finishCloseProcess runs.
    /// Idempotent: the quit that follows delivers QEvent::Quit here again.
    void closeVehicleConnections();

    /// Handle an aircast-qgc:// deep link (from the OS open-url event or a
    /// command-line argument): sets the camera video source from its query, or
    /// with host=<device> configures cameras and telemetry from an aircastd device.
    /// Applied immediately once settings are ready, otherwise deferred to init().
    void handleDeepLink(const QUrl &url);

    static QString cachedParameterMetaDataFile();
    static QString cachedAirframeMetaDataFile();

public:
    /// Perform initialize which is common to both normal application running and unit tests.
    void init();
    void shutdown();

    /// Although public, these methods are internal and should only be called by UnitTest code
    QQmlApplicationEngine *qmlAppEngine() const { return _qmlAppEngine; }
    /// UI test harnesses create their own QML engine; registering it here lets app-level
    /// messaging (showAppMessage) reach the test's MainWindow. Pass nullptr on teardown.
    void setQmlAppEngine(QQmlApplicationEngine *engine)
    {
        _qmlAppEngine = engine;
        _mainRootWindow = nullptr;    // cached from the previous engine's root object
        _uiTestMode = (engine != nullptr);
    }
    /// showRebootAppMessage() debounces repeat messages (2 min). Tests reset the
    /// debounce per-test so each one deterministically sees its own message.
    void resetRebootMessageDebounce() { _lastRebootMessageTime = QTime(); }

signals:
    void languageChanged(const QLocale &locale);

public slots:
    void qmlAttemptWindowClose();

    /// Get current language
    QLocale getCurrentLanguage() const { return _locale; }

    /// Show non-modal vehicle message to the user
    void showCriticalVehicleMessage(const QString &message);

    /// Show modal application message to the user
    void showAppMessage(const QString &message, const QString &title = QString());

    /// Show modal application message to the user about the need for a reboot. Multiple messages will be supressed if they occur
    /// one after the other.
    void showRebootAppMessage(const QString &message, const QString &title = QString());

    /// Same as showRebootAppMessage() but the dialog also includes a button which reboots the active vehicle.
    void showRebootVehicleMessage(const QString &message, const QString &title = QString());

    QGCImageProvider *qgcImageProvider();

private slots:
    /// Called when the delay timer fires to show the missing parameters warning
    void _missingParamsDisplay();
    void _qgcCurrentStableVersionDownloadComplete(bool success, const QString &localFile, const QString &errorMsg);
    static bool _parseVersionText(const QString &versionString, int &majorVersion, int &minorVersion, int &buildVersion);
    void _showDelayedAppMessages();

private:
    bool compressEvent(QEvent *event, QObject *receiver, QPostEventList *postedEvents) final;

    bool _initVideo();

    bool _initQmlRootWindow();

    /// Apply a validated aircast-qgc:// deep link to the video settings.
    void _applyDeepLink(const QUrl &url);

    /// Fetch /api/stream/config and /api/telemetry/config from an aircastd
    /// device and configure cameras and the telemetry link from them.
    void _setupFromDevice(const QString &host);
    void _applyDeviceCameras(const QString &host, const QJsonObject &config);
    void _applyDeviceTelemetry(const QString &host, const QJsonObject &config);

    /// Initialize the application for normal application boot. Or in other words we are not going to run unit tests.
    void _initForNormalAppBoot();

    QObject *_rootQmlObject();
    void _checkForNewVersion();
    bool _rebootMessageDebounced();

    bool _runningUnitTests = false;
    bool _simpleBootTest = false;
    bool _uiTestMode = false;    ///< true: QML UI test harness registered its engine via setQmlAppEngine()
    bool _fakeMobile = false;    ///< true: Fake ui into displaying mobile interface
    bool _logOutput = false;    ///< true: Log Qt debug output to file
    bool _embeddedHost = false;
    bool _connectionsClosed = false;
    quint8 _systemId = 0; ///< MAVLink system ID, 0 means not set
    QTime _lastRebootMessageTime;    ///< showRebootAppMessage() debounce state

    static constexpr int _missingParamsDelayedDisplayTimerTimeout = 1000;   ///< Timeout to wait for next missing fact to come in before display
    QTimer _missingParamsDelayedDisplayTimer;                               ///< Timer use to delay missing fact display
    QList<QPair<int,QString>> _missingParams;                               ///< List of missing parameter component id:name

    QQmlApplicationEngine *_qmlAppEngine = nullptr;
    bool _settingsUpgraded = false;    ///< true: Settings format has been upgrade to new version
    int _majorVersion = 0;
    int _minorVersion = 0;
    int _buildVersion = 0;
    QQuickWindow *_mainRootWindow = nullptr;
    QTranslator _qgcTranslatorSourceCode;           ///< translations for source code C++/Qml
    QTranslator _qgcTranslatorQtLibs;               ///< tranlsations for Qt libraries
    QLocale _locale;
    bool _error = false;
    bool _showErrorsInToolbar = false;
    QElapsedTimer _msecsElapsedTime;
    bool _videoManagerInitialized = false;
    bool _bootTestPassed = true;
    bool _settingsReady = false;        ///< true once SettingsManager is initialized and deep links can be applied
    QUrl _pendingDeepLink;              ///< aircast-qgc:// link received before settings were ready
    QNetworkAccessManager *_deviceSetupNetworkManager = nullptr; ///< Long-lived manager for _setupFromDevice(); never deleted mid-request
    int _deviceSetupGeneration = 0;     ///< Bumped on each _setupFromDevice() call so a stale reply from a superseded call can't overwrite a newer one's config

    QList<QPair<QString /* title */, QString /* message */>> _delayedAppMessages;

    class CompressedSignalList
    {
    public:
        CompressedSignalList() {}
        void add(const QMetaMethod &method);
        void remove(const QMetaMethod &method);
        bool contains(const QMetaObject *metaObject, int signalIndex);

    private:
        /// Returns a signal index that is can be compared to QMetaCallEvent.signalId
        static int _signalIndex(const QMetaMethod &method);

        QMap<const QMetaObject*, QSet<int>> _signalMap;

        Q_DISABLE_COPY(CompressedSignalList)
    };

    CompressedSignalList _compressedSignals;

    const QString _settingsVersionKey = QStringLiteral("SettingsVersion"); ///< Settings key which hold settings version

    const QString _qgcImageProviderId = QStringLiteral("QGCImages");
};

Q_DECLARE_LOGGING_CATEGORY(QGCAppMessageLog)
