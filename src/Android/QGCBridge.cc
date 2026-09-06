#include "QGCBridge.h"

#include "AndroidInterface.h"
#include "QGCBridgeCore.h"
#include "QGCLoggingCategory.h"

#include <QtCore/QJniEnvironment>
#include <QtCore/QJniObject>

QGC_LOGGING_CATEGORY(QGCBridgeLog, "qgc.android.qgcbridge")

namespace
{

jstring jniGet(JNIEnv *env, jclass clazz, jstring pathA)
{
    Q_UNUSED(clazz);
    const QString result = QGCBridgeCore::get(QJniObject(pathA).toString());
    return env->NewStringUTF(result.toUtf8().constData());
}

jstring jniSet(JNIEnv *env, jclass clazz, jstring pathA, jstring jsonA)
{
    Q_UNUSED(clazz);
    const QString result = QGCBridgeCore::set(QJniObject(pathA).toString(), QJniObject(jsonA).toString());
    return env->NewStringUTF(result.toUtf8().constData());
}

jstring jniInvoke(JNIEnv *env, jclass clazz, jstring pathA, jstring argsA)
{
    Q_UNUSED(clazz);
    const QString result = QGCBridgeCore::invoke(QJniObject(pathA).toString(), QJniObject(argsA).toString());
    return env->NewStringUTF(result.toUtf8().constData());
}

void jniWatch(JNIEnv *env, jclass clazz, jstring pathsA)
{
    Q_UNUSED(env);
    Q_UNUSED(clazz);
    QGCBridgeCore::watch(QJniObject(pathsA).toString().split(QLatin1Char(','), Qt::SkipEmptyParts));
}

void jniNotifyFontScale(JNIEnv *env, jclass clazz, jfloat scaleA)
{
    Q_UNUSED(clazz);
    AndroidInterface::jniFontScaleChanged(env, nullptr, scaleA);
}

void jniNotifySafeAreaInsets(JNIEnv *env, jclass clazz, jint leftA, jint topA, jint rightA, jint bottomA)
{
    Q_UNUSED(clazz);
    AndroidInterface::jniSafeAreaInsets(env, nullptr, leftA, topA, rightA, bottomA);
}

void jniNotifyDeepLink(JNIEnv *env, jclass clazz, jstring urlA)
{
    Q_UNUSED(clazz);
    AndroidInterface::jniDeepLink(env, nullptr, urlA);
}

} // namespace

namespace QGCBridge
{

void setNativeMethods()
{
    QGCBridgeCore::setEventHandler([](const QString &path, const QString &json) {
        QJniObject::callStaticMethod<void>(
            kJniQGCBridgeClassName, "onEvent",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            QJniObject::fromString(path).object<jstring>(),
            QJniObject::fromString(json).object<jstring>());
    });

    const JNINativeMethod javaMethods[] {
        { "get", "(Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniGet) },
        { "set", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniSet) },
        { "invoke", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniInvoke) },
        { "watch", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniWatch) },
        { "notifyFontScale", "(F)V", reinterpret_cast<void *>(jniNotifyFontScale) },
        { "notifySafeAreaInsets", "(IIII)V", reinterpret_cast<void *>(jniNotifySafeAreaInsets) },
        { "notifyDeepLink", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniNotifyDeepLink) },
    };

    QJniEnvironment jniEnv;
    (void) jniEnv.checkAndClearExceptions();

    jclass objectClass = jniEnv.findClass(kJniQGCBridgeClassName);
    if (!objectClass) {
        qCWarning(QGCBridgeLog) << "Couldn't find class:" << kJniQGCBridgeClassName;
        (void) jniEnv.checkAndClearExceptions();
        return;
    }

    const jint val = jniEnv->RegisterNatives(objectClass, javaMethods, std::size(javaMethods));
    if (val < 0) {
        qCWarning(QGCBridgeLog) << "Error registering methods:" << val;
    } else {
        qCDebug(QGCBridgeLog) << "Bridge Native Functions Registered";
    }

    (void) jniEnv.checkAndClearExceptions();
}

bool setSystemBarAppearance(bool lightBars)
{
    QJniEnvironment jniEnv;
    if (!jniEnv.findClass(kJniQGCBridgeClassName)) {
        (void) jniEnv.checkAndClearExceptions();
        return false;
    }

    QJniObject::callStaticMethod<void>(kJniQGCBridgeClassName, "onSystemBarAppearance", "(Z)V",
                                       static_cast<jboolean>(lightBars));
    (void) jniEnv.checkAndClearExceptions();
    return true;
}

} // namespace QGCBridge
