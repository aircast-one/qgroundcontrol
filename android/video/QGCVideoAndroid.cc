#include "QGCVideoC.h"

#include <android/log.h>
#include <android/native_window.h>
#include <android/native_window_jni.h>
#include <gst/gl/egl/gstgldisplay_egl.h>
#include <gst/gl/gl.h>
#include <gst/gst.h>
#include <jni.h>

#include <algorithm>
#include <array>
#include <cstdlib>
#include <mutex>
#include <string>
#include <vector>

extern "C" void gst_init_static_plugins(void);

namespace {

struct Window {
    std::mutex mutex;
    ANativeWindow *window = nullptr;
    std::vector<ANativeWindow *> retired;
};

std::array<Window, QGC_VIDEO_CHANNELS> windows;
JavaVM *javaVm = nullptr;
jobject applicationContext = nullptr;
jobject applicationClassLoader = nullptr;

Window *windowAt(int channel)
{
    return (channel >= 0 && channel < QGC_VIDEO_CHANNELS) ? &windows[static_cast<size_t>(channel)] : nullptr;
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
        if (!egl) {
            __android_log_print(ANDROID_LOG_ERROR, "qgc_video", "no shared EGL display: GL elements will make and terminate their own");
            return static_cast<GstGLDisplay *>(nullptr);
        }
        gst_gl_display_egl_set_foreign(egl, TRUE);
        return GST_GL_DISPLAY(egl);
    }();
    return display;
}

void releaseRetiredWindows(int channel)
{
    Window *const target = windowAt(channel);
    if (!target) {
        return;
    }
    const std::lock_guard<std::mutex> lock(target->mutex);
    std::for_each(target->retired.begin(), target->retired.end(), ANativeWindow_release);
    target->retired.clear();
}

void preparePipeline(int channel, void *pipeline)
{
    releaseRetiredWindows(channel);
    GstGLDisplay *const display = sharedGlDisplay();
    if (!display) {
        return;
    }
    GstContext *const context = gst_context_new(GST_GL_DISPLAY_CONTEXT_TYPE, TRUE);
    gst_context_set_gl_display(context, display);
    gst_element_set_context(GST_ELEMENT(pipeline), context);
    gst_context_unref(context);
}

}

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
        qgc_video_set_pipeline_callback(preparePipeline);
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

__attribute__((visibility("default"))) bool qgc_video_android_set_surface(JNIEnv *env, int channel, jobject surface)
{
    Window *const target = windowAt(channel);
    if (!target) {
        return false;
    }
    ANativeWindow *const next = surface ? ANativeWindow_fromSurface(env, surface) : nullptr;
    const std::lock_guard<std::mutex> lock(target->mutex);
    qgc_video_set_window(channel, next);
    if (target->window == next && next) {
        ANativeWindow_release(next);
    } else if (target->window) {
        target->retired.push_back(target->window);
    }
    target->window = next;
    return surface == nullptr || next != nullptr;
}

}
