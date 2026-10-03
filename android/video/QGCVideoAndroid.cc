#include "QGCVideoC.h"

#include <android/log.h>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <gst/gl/egl/gstgldisplay_egl.h>
#include <gst/gl/gl.h>
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

bool hardwareDecoder(const gchar *name)
{
    const bool softwareWrapper = name && (g_str_has_prefix(name, "amcviddec-omxgoogle") || g_str_has_prefix(name, "amcviddec-c2android"));
    return name && g_str_has_prefix(name, "amcviddec-") && !softwareWrapper;
}

void rankDecoders(bool preferHardware)
{
    GList *const decoders = gst_element_factory_list_get_elements(
        static_cast<GstElementFactoryListType>(GST_ELEMENT_FACTORY_TYPE_DECODER | GST_ELEMENT_FACTORY_TYPE_MEDIA_VIDEO), GST_RANK_NONE);
    for (GList *item = decoders; item; item = item->next) {
        GstPluginFeature *const feature = GST_PLUGIN_FEATURE(item->data);
        const bool preferred = hardwareDecoder(gst_plugin_feature_get_name(feature)) == preferHardware;
        if (preferred) {
            gst_plugin_feature_set_rank(feature, GST_RANK_PRIMARY + 1);
        } else if (gst_plugin_feature_get_rank(feature) >= GST_RANK_MARGINAL) {
            gst_plugin_feature_set_rank(feature, GST_RANK_NONE);
        }
    }
    gst_plugin_feature_list_free(decoders);
}

void rankNamed(const char *const *names, size_t count)
{
    for (size_t i = 0; i < count; ++i) {
        if (GstPluginFeature *const feature = gst_registry_lookup_feature(gst_registry_get(), names[i])) {
            gst_plugin_feature_set_rank(feature, GST_RANK_PRIMARY + 1);
            gst_object_unref(feature);
        }
    }
}

void preferHardwareDecoders()
{
    GList *const decoders = gst_element_factory_list_get_elements(
        static_cast<GstElementFactoryListType>(GST_ELEMENT_FACTORY_TYPE_DECODER | GST_ELEMENT_FACTORY_TYPE_MEDIA_VIDEO), GST_RANK_NONE);
    for (GList *item = decoders; item; item = item->next) {
        GstPluginFeature *const feature = GST_PLUGIN_FEATURE(item->data);
        const gchar *const name = gst_plugin_feature_get_name(feature);
        const bool softwareWrapper = name && (g_str_has_prefix(name, "amcviddec-omxgoogle") || g_str_has_prefix(name, "amcviddec-c2android"));
        if (name && g_str_has_prefix(name, "amcviddec-") && !softwareWrapper) {
            gst_plugin_feature_set_rank(feature, GST_RANK_PRIMARY + 1);
            __android_log_print(ANDROID_LOG_INFO, "qgc_video", "hardware decoder %s", name);
        }
    }
    gst_plugin_feature_list_free(decoders);
}

void setDirectory(const char *name, const std::string &value)
{
    if (!value.empty()) {
        setenv(name, value.c_str(), 1);
    }
}

GstGLDisplay *sharedGlDisplay()
{
    static GstGLDisplay *const display = [] {
        GstGLDisplayEGL *const egl = gst_gl_display_egl_new();
        if (egl) {
            gst_gl_display_egl_set_foreign(egl, TRUE);
        }
        return egl ? GST_GL_DISPLAY(egl) : nullptr;
    }();
    return display;
}

void shareGlDisplay(void *pipeline)
{
    GstGLDisplay *const display = sharedGlDisplay();
    if (!display) {
        return;
    }
    GstContext *const context = gst_context_new(GST_GL_DISPLAY_CONTEXT_TYPE, TRUE);
    gst_context_set_gl_display(context, display);
    gst_element_set_context(GST_ELEMENT(pipeline), context);
    gst_context_unref(context);
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
        preferHardwareDecoders();
        qgc_video_set_frame_callback(drawFrame);
        qgc_video_set_pipeline_callback(shareGlDisplay);
    });
    return gst_is_initialized();
}

__attribute__((visibility("default"))) void qgc_video_android_force_decoder(int option)
{
    constexpr int kSoftware = 1;
    constexpr int kVulkan = 7;
    constexpr int kHardware = 8;
    static const char *const kVulkanDecoders[] = {"vulkanh264dec", "vulkanh265dec"};
    switch (option) {
    case kSoftware:
        rankDecoders(false);
        break;
    case kHardware:
        rankDecoders(true);
        break;
    case kVulkan:
        rankNamed(kVulkanDecoders, sizeof(kVulkanDecoders) / sizeof(kVulkanDecoders[0]));
        break;
    default:
        break;
    }
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
