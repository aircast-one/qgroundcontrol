#include "QGCBridge.h"

#include "AndroidInterface.h"
#include "QGCBridgeC.h"
#include "QGCCoreC.h"
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
    char *const result = qgc_bridge_get(QJniObject(pathA).toString().toUtf8().constData());
    jstring out = env->NewStringUTF(result);
    qgc_bridge_free(result);
    return out;
}

jstring jniCoreLinkOpen(JNIEnv *env, jclass clazz, jstring configA)
{
    Q_UNUSED(clazz);
    char *const result = qgc_core_link_open(QJniObject(configA).toString().toUtf8().constData());
    jstring answer = env->NewStringUTF(result ? result : "");
    qgc_bridge_free(result);
    return answer;
}

jstring jniGetFields(JNIEnv *env, jclass clazz, jstring pathA, jstring fieldsA)
{
    Q_UNUSED(clazz);
    char *const result = qgc_bridge_get_fields(QJniObject(pathA).toString().toUtf8().constData(), QJniObject(fieldsA).toString().toUtf8().constData());
    jstring out = env->NewStringUTF(result);
    qgc_bridge_free(result);
    return out;
}

jstring jniSet(JNIEnv *env, jclass clazz, jstring pathA, jstring jsonA)
{
    Q_UNUSED(clazz);
    char *const result = qgc_bridge_set(QJniObject(pathA).toString().toUtf8().constData(), QJniObject(jsonA).toString().toUtf8().constData());
    jstring out = env->NewStringUTF(result);
    qgc_bridge_free(result);
    return out;
}

jstring jniInvoke(JNIEnv *env, jclass clazz, jstring pathA, jstring argsA)
{
    Q_UNUSED(clazz);
    char *const result = qgc_bridge_invoke(QJniObject(pathA).toString().toUtf8().constData(), QJniObject(argsA).toString().toUtf8().constData());
    jstring out = env->NewStringUTF(result);
    qgc_bridge_free(result);
    return out;
}

void jniWatch(JNIEnv *env, jclass clazz, jstring pathsA)
{
    Q_UNUSED(env);
    Q_UNUSED(clazz);
    qgc_bridge_watch(QJniObject(pathsA).toString().toUtf8().constData());
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

void relayToJava(const char *path, const char *json)
{
    QJniObject::callStaticMethod<void>(
        QGCBridge::kJniQGCBridgeClassName, "onEvent",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        QJniObject::fromString(QString::fromUtf8(path)).object<jstring>(),
        QJniObject::fromString(QString::fromUtf8(json)).object<jstring>());
}

} // namespace

namespace QGCBridge
{

void setNativeMethods()
{
    qgc_bridge_set_event_handler(relayToJava);

    const JNINativeMethod javaMethods[] {
        { "get", "(Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniGet) },
        { "getFields", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniGetFields) },
        { "coreLinkOpen", "(Ljava/lang/String;)Ljava/lang/String;", reinterpret_cast<void *>(jniCoreLinkOpen) },
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
