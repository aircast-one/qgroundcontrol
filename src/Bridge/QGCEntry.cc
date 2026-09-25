#include "QGCEntry.h"

#include "LogManager.h"
#include "Platform.h"
#include "QGCApplication.h"
#include "QGCCommandLineParser.h"
#include "QGCLoggingCategory.h"

#include <QtCore/QUrl>

#include <memory>
#include <optional>

#ifdef Q_OS_ANDROID
    #include "AndroidInterface.h"
#endif

#ifdef QGC_UNITTEST_BUILD
    #include "UnitTestList.h"
#endif

QGC_LOGGING_CATEGORY_ON(QGCEntryLog, "Main")

namespace
{
struct Runtime
{
    int argc = 0;
    std::unique_ptr<QGCApplication> app;
    std::optional<QGCCommandLineParser::CommandLineParseResult> args;
    std::optional<int> earlyExitCode;
    bool hostProvidesUI = false;
};

Runtime g_runtime;
} // namespace

int qgc_start(int argc, char *argv[])
{
    g_runtime.args = QGCCommandLineParser::parse(argc, argv);
    const auto &args = *g_runtime.args;

    if (const auto exitCode = QGCCommandLineParser::handleParseResult(args)) {
        g_runtime.earlyExitCode = *exitCode;
        return 0;
    }

    if (const auto exitCode = Platform::initialize(argc, argv, args)) {
        g_runtime.earlyExitCode = *exitCode;
        return 0;
    }

#ifdef Q_OS_ANDROID
    if (AndroidInterface::isEmbeddedHost()) {
        g_runtime.hostProvidesUI = true;
    }
#endif

    g_runtime.argc = argc;
    g_runtime.app = std::make_unique<QGCApplication>(g_runtime.argc, argv, args, g_runtime.hostProvidesUI);
    QGCApplication &app = *g_runtime.app;

    if (const std::optional<QUrl> link = Platform::deepLinkArg(argc, argv)) {
        app.handleDeepLink(*link);
    }
#if !defined(Q_OS_ANDROID) && !defined(Q_OS_IOS)
    Platform::receiveForwardedDeepLinks(&app, [&app](const QUrl &link) { app.handleDeepLink(link); });
#endif

#ifdef Q_OS_ANDROID
    const QString androidDeepLink = AndroidInterface::getLaunchDeepLink();
    if (!androidDeepLink.isEmpty()) {
        app.handleDeepLink(QUrl(androidDeepLink));
    }
#endif

    LogManager::installHandler(args.logOutput);
    Platform::setupPostApp();
    app.init();
    LogManager::applyEnvironmentLogLevel();
    return 0;
}

int qgc_run(void)
{
    if (g_runtime.earlyExitCode) {
        return *g_runtime.earlyExitCode;
    }

    const auto &args = *g_runtime.args;
    QGCApplication &app = *g_runtime.app;

    using QGCCommandLineParser::AppMode;
    switch (QGCCommandLineParser::determineAppMode(args)) {
#ifdef QGC_UNITTEST_BUILD
    case AppMode::ListTests:
    case AppMode::Test:
        return QGCUnitTest::handleTestOptions(args);
#endif
    case AppMode::BootTest:
        if (!app.bootTestPassed()) {
            qCCritical(QGCEntryLog) << "Simple boot test failed";
            return EXIT_FAILURE;
        }
        qCInfo(QGCEntryLog) << "Simple boot test completed";
        return EXIT_SUCCESS;
    case AppMode::Gui:
#ifdef Q_OS_ANDROID
        AndroidInterface::checkStoragePermissions();
#endif
        qCInfo(QGCEntryLog) << "Starting application event loop";
        return app.exec();
    }
    Q_UNREACHABLE();
}

int qgc_core_headless(void)
{
#ifdef QGC_HEADLESS_CORE
    return 1;
#else
    return 0;
#endif
}

int qgc_runs_event_loop(void)
{
    if (g_runtime.earlyExitCode || !g_runtime.args) {
        return 0;
    }
    return (QGCCommandLineParser::determineAppMode(*g_runtime.args) == QGCCommandLineParser::AppMode::Gui) ? 1 : 0;
}

void qgc_set_host_provides_ui(int provides)
{
    g_runtime.hostProvidesUI = (provides != 0);
}

void qgc_handle_deep_link(const char *url)
{
    if (!g_runtime.app || !url) {
        return;
    }
    const QUrl parsed(QString::fromUtf8(url));
    QGCApplication *const app = g_runtime.app.get();
    (void) QMetaObject::invokeMethod(app, [app, parsed]() { app->handleDeepLink(parsed); }, Qt::QueuedConnection);
}

void qgc_request_quit(void)
{
    if (!g_runtime.app) {
        return;
    }
    g_runtime.app->closeVehicleConnections();
    g_runtime.app->quit();
}

void qgc_shutdown(void)
{
    if (!g_runtime.app) {
        return;
    }
    g_runtime.app->shutdown();
    qCInfo(QGCEntryLog) << "Exiting main";
    g_runtime.app.reset();
    delete LogManager::instance();
}
