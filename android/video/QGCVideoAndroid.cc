#include "QGCVideoC.h"

#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <gst/gst.h>
#include <jni.h>

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <mutex>
#include <string>

extern "C" void gst_init_static_plugins(void);

namespace {

std::mutex windowMutex;
ANativeWindow *window = nullptr;
int windowWidth = 0;
int windowHeight = 0;
JavaVM *javaVm = nullptr;
jobject applicationContext = nullptr;
jobject applicationClassLoader = nullptr;

void drawFrame(const uint8_t *pixels, int width, int height, int stride)
{
    const std::lock_guard<std::mutex> lock(windowMutex);
    if (!window || width <= 0 || height <= 0) {
        return;
    }
    if (width != windowWidth || height != windowHeight) {
        ANativeWindow_setBuffersGeometry(window, width, height, WINDOW_FORMAT_RGBA_8888);
        windowWidth = width;
        windowHeight = height;
    }
    ANativeWindow_Buffer buffer;
    if (ANativeWindow_lock(window, &buffer, nullptr) != 0) {
        return;
    }
    const int rows = std::min(height, buffer.height);
    const int rowBytes = std::min(stride, buffer.stride * 4);
    auto *destination = static_cast<uint8_t *>(buffer.bits);
    for (int row = 0; row < rows; ++row) {
        memcpy(destination + static_cast<size_t>(row) * buffer.stride * 4, pixels + static_cast<size_t>(row) * stride, rowBytes);
    }
    ANativeWindow_unlockAndPost(window);
}

void setDirectory(const char *name, const std::string &value)
{
    if (!value.empty()) {
        setenv(name, value.c_str(), 1);
    }
}

} // namespace

extern "C" {

__attribute__((visibility("default"))) jobject gst_android_get_application_context(void)
{
    return applicationContext;
}

__attribute__((visibility("default"))) jobject gst_android_get_application_class_loader(void)
{
    return applicationClassLoader;
}

__attribute__((visibility("default"))) JavaVM *gst_android_get_java_vm(void)
{
    return javaVm;
}

__attribute__((visibility("default"))) bool qgc_video_android_init(JavaVM *vm, jobject context, jobject classLoader, const char *filesDir, const char *cacheDir)
{
    static std::once_flag once;
    std::call_once(once, [&] {
        javaVm = vm;
        applicationContext = context;
        applicationClassLoader = classLoader;
        const std::string files = filesDir ? filesDir : "";
        const std::string cache = cacheDir ? cacheDir : "";
        setDirectory("HOME", files);
        setDirectory("TMP", cache);
        setDirectory("TEMP", cache);
        setDirectory("TMPDIR", cache);
        setDirectory("XDG_CACHE_HOME", cache);
        setDirectory("XDG_CONFIG_HOME", cache);
        setDirectory("XDG_DATA_HOME", files);
        setDirectory("XDG_RUNTIME_DIR", cache);
        gst_init(nullptr, nullptr);
        gst_init_static_plugins();
        qgc_video_set_frame_callback(drawFrame);
    });
    return gst_is_initialized();
}

__attribute__((visibility("default"))) bool qgc_video_android_set_surface(JNIEnv *env, jobject surface)
{
    ANativeWindow *const next = surface ? ANativeWindow_fromSurface(env, surface) : nullptr;
    const std::lock_guard<std::mutex> lock(windowMutex);
    if (window) {
        ANativeWindow_release(window);
    }
    window = next;
    windowWidth = 0;
    windowHeight = 0;
    return surface == nullptr || next != nullptr;
}

}
