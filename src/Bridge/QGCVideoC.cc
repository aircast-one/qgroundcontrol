#include "QGCVideoC.h"

#include <mutex>
#include <string>
#include <vector>

#ifdef QGC_GST_STREAMING
#include <gst/app/gstappsink.h>
#include <gst/gst.h>
#include <gst/video/video.h>
#endif

namespace {

std::mutex frameMutex;
std::vector<uint8_t> latestFrame;
int frameWidth = 0;
int frameHeight = 0;
int frameStride = 0;
int64_t frameCount = 0;
std::string lastError;

#ifdef QGC_GST_STREAMING
GstElement *pipeline = nullptr;
GstElement *sink = nullptr;

GstFlowReturn onNewSample(GstAppSink *appsink, gpointer)
{
    GstSample *const sample = gst_app_sink_pull_sample(appsink);
    if (!sample) {
        return GST_FLOW_OK;
    }

    GstCaps *const caps = gst_sample_get_caps(sample);
    GstBuffer *const buffer = gst_sample_get_buffer(sample);
    GstVideoInfo info;
    if (caps && buffer && gst_video_info_from_caps(&info, caps)) {
        GstMapInfo map;
        if (gst_buffer_map(buffer, &map, GST_MAP_READ)) {
            const std::lock_guard<std::mutex> lock(frameMutex);
            frameWidth = GST_VIDEO_INFO_WIDTH(&info);
            frameHeight = GST_VIDEO_INFO_HEIGHT(&info);
            frameStride = static_cast<int>(GST_VIDEO_INFO_PLANE_STRIDE(&info, 0));
            latestFrame.assign(map.data, map.data + map.size);
            frameCount += 1;
            gst_buffer_unmap(buffer, &map);
        }
    }

    gst_sample_unref(sample);
    return GST_FLOW_OK;
}
#endif

} // namespace

bool qgc_video_attach_appsink(void *appsink)
{
#ifdef QGC_GST_STREAMING
    if (!appsink) {
        lastError = "no appsink given";
        return false;
    }

    GstAppSink *const adopted = GST_APP_SINK(appsink);
    GstCaps *const caps = gst_caps_new_simple("video/x-raw", "format", G_TYPE_STRING, "BGRA", nullptr);
    gst_app_sink_set_caps(adopted, caps);
    gst_caps_unref(caps);
    gst_app_sink_set_max_buffers(adopted, 1);
    gst_app_sink_set_drop(adopted, TRUE);

    GstAppSinkCallbacks callbacks = {};
    callbacks.new_sample = onNewSample;
    gst_app_sink_set_callbacks(adopted, &callbacks, nullptr, nullptr);

    {
        const std::lock_guard<std::mutex> lock(frameMutex);
        latestFrame.clear();
        frameWidth = 0;
        frameHeight = 0;
        frameStride = 0;
        frameCount = 0;
    }

    lastError.clear();
    return true;
#else
    (void)appsink;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_detach_appsink(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    latestFrame.clear();
    frameWidth = 0;
    frameHeight = 0;
    frameStride = 0;
    frameCount = 0;
}

bool qgc_video_available(void)
{
#ifdef QGC_GST_STREAMING
    return true;
#else
    return false;
#endif
}

bool qgc_video_start(const char *pipelineDescription)
{
#ifdef QGC_GST_STREAMING
    if (!pipelineDescription) {
        lastError = "no pipeline given";
        return false;
    }

    qgc_video_stop();

    if (!gst_is_initialized()) {
        gst_init(nullptr, nullptr);
    }

    GError *error = nullptr;
    pipeline = gst_parse_launch(pipelineDescription, &error);
    if (!pipeline) {
        lastError = error ? error->message : "gst_parse_launch failed";
        if (error) {
            g_error_free(error);
        }
        return false;
    }
    if (error) {
        g_error_free(error);
    }

    sink = gst_bin_get_by_name(GST_BIN(pipeline), "nativesink");
    if (!sink) {
        lastError = "the pipeline has no appsink named nativesink";
        qgc_video_stop();
        return false;
    }

    GstCaps *const caps = gst_caps_new_simple("video/x-raw", "format", G_TYPE_STRING, "BGRA", nullptr);
    gst_app_sink_set_caps(GST_APP_SINK(sink), caps);
    gst_caps_unref(caps);
    gst_app_sink_set_max_buffers(GST_APP_SINK(sink), 1);
    gst_app_sink_set_drop(GST_APP_SINK(sink), TRUE);

    GstAppSinkCallbacks callbacks = {};
    callbacks.new_sample = onNewSample;
    gst_app_sink_set_callbacks(GST_APP_SINK(sink), &callbacks, nullptr, nullptr);

    if (gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) {
        lastError = "the pipeline refused to play";
        qgc_video_stop();
        return false;
    }

    lastError.clear();
    return true;
#else
    (void)pipelineDescription;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_stop(void)
{
#ifdef QGC_GST_STREAMING
    if (pipeline) {
        gst_element_set_state(pipeline, GST_STATE_NULL);
    }
    if (sink) {
        gst_object_unref(sink);
        sink = nullptr;
    }
    if (pipeline) {
        gst_object_unref(pipeline);
        pipeline = nullptr;
    }
#endif
    const std::lock_guard<std::mutex> lock(frameMutex);
    latestFrame.clear();
    frameWidth = 0;
    frameHeight = 0;
    frameStride = 0;
    frameCount = 0;
}

bool qgc_video_running(void)
{
#ifdef QGC_GST_STREAMING
    return pipeline != nullptr;
#else
    return false;
#endif
}

int qgc_video_width(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameWidth;
}

int qgc_video_height(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameHeight;
}

int64_t qgc_video_frames(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameCount;
}

const char *qgc_video_last_error(void)
{
    return lastError.c_str();
}

bool qgc_video_copy_frame(void *destination, int capacity, int *width, int *height, int *stride)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    if (latestFrame.empty() || !destination || capacity < static_cast<int>(latestFrame.size())) {
        return false;
    }
    memcpy(destination, latestFrame.data(), latestFrame.size());
    if (width) {
        *width = frameWidth;
    }
    if (height) {
        *height = frameHeight;
    }
    if (stride) {
        *stride = frameStride;
    }
    return true;
}
