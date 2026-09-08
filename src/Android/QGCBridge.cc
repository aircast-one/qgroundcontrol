#include "QGCBridge.h"

#include "AndroidInterface.h"
#include "QGCBridgeCore.h"
#include "QGCVideoC.h"
#include "QGCLoggingCategory.h"

#include <android/native_window_jni.h>

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

jint jniVideoWidth(JNIEnv *, jclass)
{
    return qgc_video_width();
}

jint jniVideoHeight(JNIEnv *, jclass)
{
    return qgc_video_height();
}

jlong jniVideoFrames(JNIEnv *, jclass)
{
    return static_cast<jlong>(qgc_video_frames());
}

jboolean jniVideoCopyFrame(JNIEnv *env, jclass, jobject buffer)
{
    if (!buffer) {
        return JNI_FALSE;
    }

    void *const destination = env->GetDirectBufferAddress(buffer);
    const jlong capacity = env->GetDirectBufferCapacity(buffer);
    if (!destination || (capacity <= 0)) {
        return JNI_FALSE;
    }

    int width = 0;
    int height = 0;
    int stride = 0;
    return qgc_video_copy_frame(destination, static_cast<int>(capacity), &width, &height, &stride)
        ? JNI_TRUE
        : JNI_FALSE;
}

ANativeWindow *heldWindow = nullptr;

jboolean jniVideoSetSurface(JNIEnv *env, jclass, jobject surface)
{
    ANativeWindow *const window = surface ? ANativeWindow_fromSurface(env, surface) : nullptr;
    const bool applied = qgc_video_set_window(window);
    if (!applied) {
        if (window) {
            ANativeWindow_release(window);
        }
        return JNI_FALSE;
    }

    if (heldWindow) {
        ANativeWindow_release(heldWindow);
    }
    heldWindow = window;
    return JNI_TRUE;
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
        { "nativeWatch", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniWatch) },
        { "notifyFontScale", "(F)V", reinterpret_cast<void *>(jniNotifyFontScale) },
        { "notifySafeAreaInsets", "(IIII)V", reinterpret_cast<void *>(jniNotifySafeAreaInsets) },
        { "notifyDeepLink", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniNotifyDeepLink) },
        { "videoWidth", "()I", reinterpret_cast<void *>(jniVideoWidth) },
        { "videoHeight", "()I", reinterpret_cast<void *>(jniVideoHeight) },
        { "videoFrames", "()J", reinterpret_cast<void *>(jniVideoFrames) },
        { "videoCopyFrame", "(Ljava/nio/ByteBuffer;)Z", reinterpret_cast<void *>(jniVideoCopyFrame) },
        { "videoSetSurface", "(Landroid/view/Surface;)Z", reinterpret_cast<void *>(jniVideoSetSurface) },
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
