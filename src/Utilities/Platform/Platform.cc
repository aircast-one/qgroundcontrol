#include "Platform.h"
#include "qgc_version.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QProcessEnvironment>
#include <QtNetwork/QLocalServer>
#include <QtNetwork/QLocalSocket>
#ifndef QGC_HEADLESS_CORE
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGRendererInterface>
#endif

#include <algorithm>
#include <cstdio>
#include <span>

#include "QGCCommandLineParser.h"

#if !defined(Q_OS_IOS) && !defined(Q_OS_ANDROID)
    #include "RunGuard.h"
    #include "SignalHandler.h"
#endif

#if (defined(Q_OS_LINUX) || defined(Q_OS_FREEBSD)) && !defined(Q_OS_ANDROID)
    #include <unistd.h>
    #include <sys/types.h>
    #include <sys/wait.h>
#endif

#if defined(Q_OS_MACOS)
    #include <CoreFoundation/CoreFoundation.h>
#elif defined(Q_OS_WIN)
    #include <qt_windows.h>
    #include <iostream>
    #include <iterator>
    #include <cwchar>
    #if defined(_MSC_VER)
        #include <crtdbg.h>
        #include <stdlib.h>
    #endif
#endif

namespace {

#if (defined(Q_OS_LINUX) || defined(Q_OS_FREEBSD)) && !defined(Q_OS_ANDROID)
static void showLinuxErrorDialog(const QByteArray& msg)
{
    const pid_t pid = fork();
    if (pid == 0) {
        const QByteArray zenityText = QByteArrayLiteral("--text=") + msg;
        execlp("zenity", "zenity", "--error", "--title=Error", zenityText.constData(), nullptr);
        execlp("kdialog", "kdialog", "--error", msg.constData(), nullptr);
        execlp("xmessage", "xmessage", "-center", msg.constData(), nullptr);
        _exit(1);
    } else if (pid > 0) {
        int status = 0;
        (void) waitpid(pid, &status, 0);
    }
    fprintf(stderr, "Error: %s\n", msg.constData());
}
#endif

#if defined(Q_OS_MACOS)
void disableAppNapViaInfoDict()
{
    CFBundleRef bundle = CFBundleGetMainBundle();
    if (!bundle) {
        return;
    }
    CFMutableDictionaryRef infoDict = const_cast<CFMutableDictionaryRef>(CFBundleGetInfoDictionary(bundle));
    if (infoDict) {
        CFDictionarySetValue(infoDict, CFSTR("NSAppSleepDisabled"), kCFBooleanTrue);
    }
}
#endif

#if defined(Q_OS_WIN)

#if defined(_MSC_VER)

#if defined(_DEBUG)
int __cdecl WindowsCrtReportHook(int reportType, char* message, int* returnValue)
{
    if (message) {
        std::cerr << message << std::endl;
    }
    if (reportType == _CRT_ASSERT) {
        if (returnValue) {
            *returnValue = 0;
        }
        return 1;
    }
    return 0;
}
#endif

void __cdecl WindowsPurecallHandler()
{
    (void) OutputDebugStringW(L"QGC: _purecall\n");
}

void WindowsInvalidParameterHandler([[maybe_unused]] const wchar_t* expression,
                                    [[maybe_unused]] const wchar_t* function,
                                    [[maybe_unused]] const wchar_t* file,
                                    [[maybe_unused]] unsigned int line,
                                    [[maybe_unused]] uintptr_t pReserved)
{

}
#endif

LPTOP_LEVEL_EXCEPTION_FILTER g_prevUef = nullptr;

LONG WINAPI WindowsUnhandledExceptionFilter(EXCEPTION_POINTERS* ep)
{
    const DWORD code = (ep && ep->ExceptionRecord) ? ep->ExceptionRecord->ExceptionCode : 0;
    wchar_t buf[128] = {};
#if defined(_MSC_VER)
    (void) _snwprintf_s(buf, _TRUNCATE, L"QGC: unhandled SEH 0x%08lX\n", static_cast<unsigned long>(code));
#else
    (void) swprintf(buf, static_cast<int>(std::size(buf)), L"QGC: unhandled SEH 0x%08lX\n", static_cast<unsigned long>(code));
#endif
    (void) OutputDebugStringW(buf);

    const HANDLE h = GetStdHandle(STD_ERROR_HANDLE);
    if (h && (h != INVALID_HANDLE_VALUE)) {
        DWORD ignored = 0;
        const char narrow[] = "QGC: unhandled SEH\n";
        (void) WriteFile(h, narrow, (DWORD)sizeof(narrow) - 1, &ignored, nullptr);
    }

    return EXCEPTION_EXECUTE_HANDLER;
}

void setWindowsErrorModes(bool quietWindowsAsserts)
{
    (void) SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    g_prevUef = SetUnhandledExceptionFilter(WindowsUnhandledExceptionFilter);

#if defined(_MSC_VER)
    (void) _set_invalid_parameter_handler(WindowsInvalidParameterHandler);
    (void) _set_purecall_handler(WindowsPurecallHandler);

    if (quietWindowsAsserts) {
        (void) _CrtSetReportMode(_CRT_ASSERT, _CRTDBG_MODE_DEBUG);
        (void) _CrtSetReportMode(_CRT_ERROR,  _CRTDBG_MODE_DEBUG);
        (void) _CrtSetReportMode(_CRT_WARN,   _CRTDBG_MODE_DEBUG);
        (void) _CrtSetReportHook2(_CRT_RPTHOOK_INSTALL, WindowsCrtReportHook);
        (void) _set_abort_behavior(0, _WRITE_ABORT_MSG | _CALL_REPORTFAULT);
        (void) _set_error_mode(_OUT_TO_STDERR);
    }
#else
    Q_UNUSED(quietWindowsAsserts);
#endif
}
#endif

}

std::optional<int> Platform::initialize(int argc, char* argv[],
                                         const QGCCommandLineParser::CommandLineParseResult& args)
{
#if (defined(Q_OS_LINUX) || defined(Q_OS_FREEBSD)) && !defined(Q_OS_ANDROID)
    if (isRunningAsRoot()) {
        return showRootError(argc, argv);
    }
#endif

#if !defined(Q_OS_ANDROID) && !defined(Q_OS_IOS)
    if (!checkSingleInstance(allowsMultipleInstances(args))) {
        const std::optional<QUrl> link = deepLinkArg(argc, argv);
        if (link && forwardDeepLink(*link)) {
            return 0;
        }
        return showMultipleInstanceError(argc, argv);
    }
#else
    Q_UNUSED(argc);
    Q_UNUSED(argv);
#endif

#ifdef Q_OS_UNIX
#ifndef Q_OS_ANDROID
    if (!qEnvironmentVariableIsSet("QT_ASSUME_STDERR_HAS_CONSOLE")) {
        (void) qputenv("QT_ASSUME_STDERR_HAS_CONSOLE", "1");
    }
    if (!qEnvironmentVariableIsSet("QT_FORCE_STDERR_LOGGING")) {
        (void) qputenv("QT_FORCE_STDERR_LOGGING", "1");
    }
#endif
#endif

#ifdef Q_OS_WIN
    if (!qEnvironmentVariableIsSet("QT_WIN_DEBUG_CONSOLE")) {
        (void) qputenv("QT_WIN_DEBUG_CONSOLE", "attach");
    }
    if (qEnvironmentVariable("QSG_RHI_BACKEND").compare(QLatin1String("d3d12"), Qt::CaseInsensitive) == 0) {
#ifndef QGC_HEADLESS_CORE
        QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12);
#endif
    }
    setWindowsErrorModes(args.quietWindowsAsserts);
#endif

#ifdef Q_OS_MACOS
    disableAppNapViaInfoDict();
#endif

#ifdef QGC_UNITTEST_BUILD
    if ((args.runningUnitTests || args.listTests) && !args.onscreen) {
        if (!qEnvironmentVariableIsSet("QT_QPA_PLATFORM")) {
            (void) qputenv("QT_QPA_PLATFORM", "offscreen");
        }
    }
#endif

    if (args.useSwRast) {
#ifndef QGC_HEADLESS_CORE
        QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
#endif
        QCoreApplication::setAttribute(Qt::AA_UseSoftwareOpenGL);
    }
#if defined(Q_OS_LINUX) && !defined(Q_OS_ANDROID) && \
    (defined(QGC_HAS_GST_GLMEMORY_GPU_PATH) || defined(QGC_HAS_GST_DMABUF_GPU_PATH))
    else if (!qEnvironmentVariableIsSet("QSG_RHI_BACKEND")) {
#ifndef QGC_HEADLESS_CORE
        QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);
#endif
    }
#endif

#if defined(QGC_HAS_GST_GLMEMORY_GPU_PATH) || defined(QGC_HAS_GST_DMABUF_GPU_PATH)
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
#endif
    QCoreApplication::setAttribute(Qt::AA_CompressTabletEvents);

    return std::nullopt;
}

void Platform::setupPostApp()
{
#if !defined(Q_OS_IOS) && !defined(Q_OS_ANDROID)
    SignalHandler* signalHandler = new SignalHandler(QCoreApplication::instance());
    (void) signalHandler->setupSignalHandlers();
#endif
}

#if (defined(Q_OS_LINUX) || defined(Q_OS_FREEBSD)) && !defined(Q_OS_ANDROID)
bool Platform::isRunningAsRoot()
{
    return ::getuid() == 0;
}

int Platform::showRootError([[maybe_unused]] int argc, [[maybe_unused]] char *argv[])
{
    const QString message = QCoreApplication::translate("main",
        "You are running %1 as root. "
        "You should not do this since it will cause other issues with %1. "
        "%1 will now exit.").arg(QLatin1String(QGC_APP_NAME));
    showLinuxErrorDialog(message.toLocal8Bit());
    return -1;
}
#endif

#if !defined(Q_OS_ANDROID) && !defined(Q_OS_IOS)
int Platform::showMultipleInstanceError([[maybe_unused]] int argc, [[maybe_unused]] char *argv[])
{
    const QString message = QCoreApplication::translate("main",
        "A second instance of %1 is already running. "
        "Please close the other instance and try again.").arg(QLatin1String(QGC_APP_NAME));
#if defined(Q_OS_MACOS)
    fprintf(stderr, "Error: %s\n", message.toLocal8Bit().constData());
    CFStringRef cfMessage = CFStringCreateWithCString(nullptr, message.toUtf8().constData(), kCFStringEncodingUTF8);
    CFUserNotificationDisplayAlert(0, kCFUserNotificationStopAlertLevel,
                                   nullptr, nullptr, nullptr,
                                   CFSTR("Error"), cfMessage,
                                   nullptr, nullptr, nullptr, nullptr);
    CFRelease(cfMessage);
#elif defined(Q_OS_WIN)
    fprintf(stderr, "Error: %s\n", message.toLocal8Bit().constData());
    MessageBoxW(nullptr, message.toStdWString().c_str(), L"Error", MB_OK | MB_ICONERROR);
#else
    showLinuxErrorDialog(message.toLocal8Bit());
#endif
    return -1;
}

bool Platform::allowsMultipleInstances(const QGCCommandLineParser::CommandLineParseResult &args)
{
    return args.allowMultiple || args.runningUnitTests || args.listTests;
}

bool Platform::checkSingleInstance(bool allowMultiple)
{
    if (allowMultiple) {
        return true;
    }

    static const QString runguardString = QStringLiteral("%1 RunGuardKey").arg(QLatin1String(QGC_APP_NAME));
    static RunGuard guard(runguardString);
    return guard.tryToRun();
}

namespace {
QString deepLinkServerName()
{
    return QStringLiteral("%1 DeepLink").arg(QLatin1String(QGC_APP_NAME));
}
}

bool Platform::forwardDeepLink(const QUrl &link)
{
    int argc = 0;
    const QCoreApplication app(argc, nullptr);
    QLocalSocket socket;
    socket.connectToServer(deepLinkServerName());
    if (!socket.waitForConnected(2000)) {
        return false;
    }
    socket.write(link.toEncoded() + '\n');
    const bool written = socket.waitForBytesWritten(2000);
    socket.disconnectFromServer();
    return written;
}

void Platform::receiveForwardedDeepLinks(QObject *owner, std::function<void(const QUrl &)> onLink)
{
    auto *server = new QLocalServer(owner);
    (void) QLocalServer::removeServer(deepLinkServerName());
    if (!server->listen(deepLinkServerName())) {
        qWarning() << "deep link server failed to listen:" << server->errorString();
        return;
    }
    (void) QObject::connect(server, &QLocalServer::newConnection, server, [server, onLink]() {
        QLocalSocket *socket = server->nextPendingConnection();
        const auto drain = [socket, onLink]() {
            if (socket->canReadLine()) {
                onLink(QUrl::fromEncoded(socket->readLine().trimmed()));
                socket->disconnectFromServer();
            }
        };
        (void) QObject::connect(socket, &QLocalSocket::readyRead, socket, drain);
        (void) QObject::connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
        drain();
    });
}
#endif

std::optional<QUrl> Platform::deepLinkArg(int argc, char *argv[])
{
    const auto args = std::span(argv, static_cast<size_t>(argc)).subspan(argc > 0 ? 1 : 0);
    const auto it = std::ranges::find_if(args, [](const char *arg) {
        return QString::fromLocal8Bit(arg).startsWith(QStringLiteral("aircast-qgc://"));
    });
    return it == args.end() ? std::nullopt : std::optional<QUrl>(QUrl(QString::fromLocal8Bit(*it)));
}
