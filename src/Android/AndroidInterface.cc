#include "AndroidInterface.h"

#include <QAndroidScreen.h>
#include <QtAndroidHelpers/QAndroidPartialWakeLocker.h>
#include <QtAndroidHelpers/QAndroidWiFiLocker.h>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QJniEnvironment>
#include <QtCore/QJniObject>
#include <QtCore/QMetaObject>
#include <QtCore/QSharedPointer>
#include <QtCore/QStandardPaths>
#include <QtCore/QUrl>

#include <mutex>

#include "AppSettings.h"
#include "QGCBridge.h"
#include "ScreenToolsController.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "SettingsFact.h"
#include "SettingsManager.h"

QGC_LOGGING_CATEGORY(AndroidInterfaceLog, "Android.AndroidInterface")

namespace AndroidInterface {

static std::function<void(const QString&)> s_importCallback;

static void jniLogDebug(JNIEnv*, jobject, jstring message)
{
    qCDebug(AndroidInterfaceLog) << QJniObject(message).toString();
}

static void jniLogWarning(JNIEnv*, jobject, jstring message)
{
    qCWarning(AndroidInterfaceLog) << QJniObject(message).toString();
}

static void jniStoragePermissionsResult(JNIEnv*, jobject, jboolean granted)
{
    if (!qgcApp()) {
        return;
    }

    if (!granted) {
        qCWarning(AndroidInterfaceLog) << "Storage permission request denied; disabling save to SD card";

        (void)QMetaObject::invokeMethod(
            qgcApp(),
            []() {
                SettingsManager* const settingsManager = SettingsManager::instance();
                if (!settingsManager) {
                    return;
                }

                AppSettings* const appSettings = settingsManager->appSettings();
                if (!appSettings) {
                    return;
                }

                if (!appSettings->androidDontSaveToSDCard()->rawValue().toBool()) {
                    appSettings->androidDontSaveToSDCard()->setRawValue(true);
                }
            },
            Qt::QueuedConnection);
        return;
    }

    (void)QMetaObject::invokeMethod(
        qgcApp(),
        []() {
            SettingsManager* const settingsManager = SettingsManager::instance();
            if (!settingsManager) {
                return;
            }

            AppSettings* const appSettings = settingsManager->appSettings();
            if (!appSettings || appSettings->androidDontSaveToSDCard()->rawValue().toBool()) {
                return;
            }

            SettingsFact* const savePathFact = qobject_cast<SettingsFact*>(appSettings->savePath());
            if (!savePathFact) {
                return;
            }

            const QString appName = QCoreApplication::applicationName();
            const QString currentSavePath = savePathFact->rawValue().toString();
            const QString internalBasePath = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
            const QString internalSavePath = QDir(internalBasePath).filePath(appName);

            if (!currentSavePath.isEmpty() && (currentSavePath != internalSavePath)) {
                return;
            }

            const QString sdCardRootPath = getSDCardPath();
            if (sdCardRootPath.isEmpty() || !QDir(sdCardRootPath).exists() || !QFileInfo(sdCardRootPath).isWritable()) {
                return;
            }

            const QString sdSavePath = QDir(sdCardRootPath).filePath(appName);
            if (currentSavePath != sdSavePath) {
                qCDebug(AndroidInterfaceLog) << "Applying SD card save path after permission grant:" << sdSavePath;
                savePathFact->setRawValue(sdSavePath);
            }
        },
        Qt::QueuedConnection);
}

static void jniOnImportResult(JNIEnv* env, jobject, jstring filePathA)
{
    const char* const filePathCStr = env->GetStringUTFChars(filePathA, nullptr);
    const QString filePath = QString::fromUtf8(filePathCStr);
    env->ReleaseStringUTFChars(filePathA, filePathCStr);
    (void)QJniEnvironment::checkAndClearExceptions(env);
    auto callback = std::move(s_importCallback);
    if (!callback) {
        return;
    }
    callback(filePath);
}

void jniDeepLink(JNIEnv *envA, jobject thizA, jstring urlA)
{
    Q_UNUSED(thizA);

    const char * const stringL = envA->GetStringUTFChars(urlA, nullptr);
    const QString url = QString::fromUtf8(stringL);
    envA->ReleaseStringUTFChars(urlA, stringL);
    (void) QJniEnvironment::checkAndClearExceptions(envA);

    QGCApplication *app = qgcApp();
    if (!app) {
        qCWarning(AndroidInterfaceLog) << "Deep link received before app ready" << url;
        return;
    }

    (void) QMetaObject::invokeMethod(app, [app, url]() {
        app->handleDeepLink(QUrl(url));
    }, Qt::QueuedConnection);
}

namespace {
struct SafeAreaInsets {
    int left = 0;
    int top = 0;
    int right = 0;
    int bottom = 0;
    bool known = false;
};

std::mutex safeAreaMutex;
AndroidInterface::SafeAreaHandler safeAreaHandler;
SafeAreaInsets safeAreaInsets;
}  // namespace

void jniFontScaleChanged(JNIEnv*, jobject, jfloat scale)
{
    ScreenToolsController::setSystemFontScale(scale);
}

void jniSafeAreaInsets(JNIEnv*, jobject, jint left, jint top, jint right, jint bottom)
{
    qCDebug(AndroidInterfaceLog) << "Window insets" << left << top << right << bottom;

    SafeAreaHandler handler;
    {
        const std::lock_guard<std::mutex> lock(safeAreaMutex);
        safeAreaInsets = SafeAreaInsets{left, top, right, bottom, true};
        handler = safeAreaHandler;
    }

    if (handler) {
        handler(left, top, right, bottom);
    }
}

void setNativeMethods()
{
    qCDebug(AndroidInterfaceLog) << "Registering Native Functions";

    const JNINativeMethod javaMethods[]{
        {"qgcLogDebug", "(Ljava/lang/String;)V", reinterpret_cast<void*>(jniLogDebug)},
        {"qgcLogWarning", "(Ljava/lang/String;)V", reinterpret_cast<void*>(jniLogWarning)},
        {"nativeStoragePermissionsResult", "(Z)V", reinterpret_cast<void*>(jniStoragePermissionsResult)},
        {"onImportResult", "(Ljava/lang/String;)V", reinterpret_cast<void*>(jniOnImportResult)},
        {"nativeDeepLink", "(Ljava/lang/String;)V", reinterpret_cast<void*>(jniDeepLink)},
        {"nativeSafeAreaInsets", "(IIII)V", reinterpret_cast<void*>(jniSafeAreaInsets)},
        {"nativeFontScaleChanged", "(F)V", reinterpret_cast<void*>(jniFontScaleChanged)}};

    QJniEnvironment env;
    if (!env.registerNativeMethods(kJniQGCActivityClassName, javaMethods, std::size(javaMethods))) {
        qCWarning(AndroidInterfaceLog) << "Failed to register native methods for" << kJniQGCActivityClassName;
    } else {
        qCDebug(AndroidInterfaceLog) << "Native Functions Registered";
    }
}

QString getLaunchDeepLink()
{
    const QJniObject activity = QNativeInterface::QAndroidApplication::context();
    if (!activity.isValid()) {
        return QString();
    }

    const QJniObject intent = activity.callObjectMethod("getIntent", "()Landroid/content/Intent;");
    (void) QJniEnvironment().checkAndClearExceptions();
    if (!intent.isValid()) {
        return QString();
    }

    const QJniObject action = intent.callObjectMethod("getAction", "()Ljava/lang/String;");
    if (!action.isValid() || action.toString() != QStringLiteral("android.intent.action.VIEW")) {
        return QString();
    }

    const QJniObject data = intent.callObjectMethod("getData", "()Landroid/net/Uri;");
    if (!data.isValid()) {
        return QString();
    }

    const QJniObject url = data.callObjectMethod("toString", "()Ljava/lang/String;");
    (void) QJniEnvironment().checkAndClearExceptions();
    if (!url.isValid()) {
        return QString();
    }

    return url.toString();
}

bool checkStoragePermissions()
{
    const bool hasPermission =
        QJniObject::callStaticMethod<jboolean>(kJniQGCActivityClassName, "checkStoragePermissions", "()Z");
    QJniEnvironment env;
    if (env.checkAndClearExceptions()) {
        qCWarning(AndroidInterfaceLog) << "Exception in checkStoragePermissions";
        return false;
    }

    if (hasPermission) {
        qCDebug(AndroidInterfaceLog) << "Storage permissions granted";
    } else {
        qCWarning(AndroidInterfaceLog) << "Storage permissions not granted";
    }

    return hasPermission;
}

QString getSDCardPath()
{
    if (!checkStoragePermissions()) {
        qCWarning(AndroidInterfaceLog) << "Storage Permission Denied";
        return QString();
    }

    const QJniObject result =
        QJniObject::callStaticObjectMethod(kJniQGCActivityClassName, "getSDCardPath", "()Ljava/lang/String;");
    QJniEnvironment env;
    if (env.checkAndClearExceptions()) {
        qCWarning(AndroidInterfaceLog) << "Exception in getSDCardPath";
        return QString();
    }
    if (!result.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Call to java getSDCardPath failed: Invalid Result";
        return QString();
    }

    return result.toString();
}

void openFileImportDialog(const QString& destPath, std::function<void(const QString&)> callback)
{
    s_importCallback = std::move(callback);

    const QJniObject jDestPath = QJniObject::fromString(destPath);
    QJniObject::callStaticMethod<void>(
        kJniQGCActivityClassName,
        "openFileImportDialog",
        "(Ljava/lang/String;)V",
        jDestPath.object<jstring>());

    QJniEnvironment env;
    if (env.checkAndClearExceptions()) {
        qCWarning(AndroidInterfaceLog) << "Exception in openFileImportDialog";
        if (s_importCallback) {
            auto cb = std::move(s_importCallback);
            cb(QString());
        }
    }
}

static QSharedPointer<QLocks::QLockBase> s_partialWakeLock;
static QSharedPointer<QLocks::QLockBase> s_wifiLock;

void setKeepScreenOn(bool on)
{
    if (!QAndroidScreen::instance()) {
        new QAndroidScreen(QCoreApplication::instance());
    }
    QAndroidScreen::instance()->keepScreenOn(on);

    if (on) {
        s_partialWakeLock = QAndroidPartialWakeLocker::instance().getLock();
        s_wifiLock = QAndroidWiFiLocker::instance().getLock();
    } else {
        s_partialWakeLock.reset();
        s_wifiLock.reset();
    }
}

void setSafeAreaHandler(SafeAreaHandler handler)
{
    SafeAreaHandler replay;
    SafeAreaInsets insets;
    {
        const std::lock_guard<std::mutex> lock(safeAreaMutex);
        safeAreaHandler = std::move(handler);
        insets = safeAreaInsets;
        if (safeAreaHandler && insets.known) {
            replay = safeAreaHandler;
        }
    }

    if (replay) {
        qCDebug(AndroidInterfaceLog) << "Replaying window insets to new handler";
        replay(insets.left, insets.top, insets.right, insets.bottom);
    }
}

bool isEmbeddedHost()
{
    QJniEnvironment env;
    const QJniObject context = QNativeInterface::QAndroidApplication::context();
    const jclass qtActivityClass = env.findClass("org/qtproject/qt/android/bindings/QtActivity");
    return context.isValid() && qtActivityClass && !env->IsInstanceOf(context.object(), qtActivityClass);
}

qreal systemFontScale()
{
    const QJniObject context = QNativeInterface::QAndroidApplication::context();
    if (!context.isValid()) {
        qCWarning(AndroidInterfaceLog) << "No Android context; assuming font scale 1.0";
        return 1.0;
    }
    const QJniObject resources = context.callObjectMethod("getResources", "()Landroid/content/res/Resources;");
    (void)QJniEnvironment().checkAndClearExceptions();
    if (!resources.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Resources unavailable; assuming font scale 1.0";
        return 1.0;
    }
    const QJniObject configuration = resources.callObjectMethod("getConfiguration", "()Landroid/content/res/Configuration;");
    (void)QJniEnvironment().checkAndClearExceptions();
    if (!configuration.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Configuration unavailable; assuming font scale 1.0";
        return 1.0;
    }
    const float scale = configuration.getField<jfloat>("fontScale");
    if (QJniEnvironment().checkAndClearExceptions() || !(scale > 0)) {
        qCWarning(AndroidInterfaceLog) << "Configuration.fontScale unreadable; assuming font scale 1.0";
        return 1.0;
    }
    return scale;
}

QColor systemColor(const QString& resourceName)
{
    const QJniObject activity = QNativeInterface::QAndroidApplication::context();
    if (!activity.isValid()) {
        return QColor();
    }

    const QJniObject resources = activity.callObjectMethod("getResources", "()Landroid/content/res/Resources;");
    if (!resources.isValid()) {
        return QColor();
    }

    const jint resourceId = resources.callMethod<jint>(
        "getIdentifier",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        QJniObject::fromString(resourceName).object<jstring>(),
        QJniObject::fromString(QStringLiteral("color")).object<jstring>(),
        QJniObject::fromString(QStringLiteral("android")).object<jstring>());
    (void)QJniEnvironment().checkAndClearExceptions();

    if (resourceId == 0) {
        return QColor();
    }

    const jint argb = activity.callMethod<jint>("getColor", "(I)I", resourceId);
    if (QJniEnvironment().checkAndClearExceptions()) {
        return QColor();
    }

    return QColor::fromRgb(static_cast<QRgb>(static_cast<unsigned int>(argb)));
}

void setSystemBarAppearance(bool lightBars)
{
    if (QGCBridge::setSystemBarAppearance(lightBars)) {
        return;
    }

    QJniObject::callStaticMethod<void>(kJniQGCActivityClassName, "setSystemBarAppearance", "(Z)V", static_cast<jboolean>(lightBars));
    (void)QJniEnvironment().checkAndClearExceptions();
}

}  // namespace AndroidInterface
