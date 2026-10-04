#include <android/log.h>
#include <gst/gst.h>
#include <gst/rtsp-server/rtsp-server.h>

#include <string>
#include <thread>

namespace {

constexpr const char *kTag = "QGCMockVideo";
constexpr const char *kHost = "127.0.0.1";
constexpr const char *kTestSource =
    "videotestsrc is-live=true pattern=ball ! "
    "video/x-raw,width=1280,height=720,framerate=30/1 ! "
    "videoconvert";
constexpr const char *kH264Encoder = "x264enc tune=zerolatency bitrate=2000 key-int-max=30";
constexpr const char *kH265Encoder = "x265enc tune=zerolatency bitrate=2000";

enum StreamType {
    RtpUdpH264 = 1,
    RtpUdpH265 = 2,
    RtspH264 = 3,
    MpegTsUdp = 4,
    MpegTsTcp = 5,
};

struct Server {
    GstElement *pipeline = nullptr;
    GstRTSPServer *rtsp = nullptr;
    GMainContext *context = nullptr;
    GMainLoop *loop = nullptr;
    std::thread thread;
    std::string uri;
};

GstBusSyncReply logProblems(GstBus *, GstMessage *message, gpointer)
{
    if (GST_MESSAGE_TYPE(message) == GST_MESSAGE_ERROR || GST_MESSAGE_TYPE(message) == GST_MESSAGE_WARNING) {
        GError *error = nullptr;
        gchar *detail = nullptr;
        (GST_MESSAGE_TYPE(message) == GST_MESSAGE_ERROR ? gst_message_parse_error : gst_message_parse_warning)(message, &error, &detail);
        __android_log_print(ANDROID_LOG_WARN, kTag, "%s: %s (%s)", GST_OBJECT_NAME(message->src), error ? error->message : "", detail ? detail : "");
        g_clear_error(&error);
        g_free(detail);
    }
    return GST_BUS_PASS;
}

bool startPipeline(Server *server, const std::string &launch)
{
    GError *error = nullptr;
    GstElement *pipeline = gst_parse_launch(launch.c_str(), &error);
    if (error) {
        __android_log_print(ANDROID_LOG_WARN, kTag, "pipeline %s: %s", launch.c_str(), error->message);
        g_clear_error(&error);
        if (pipeline) {
            gst_object_unref(pipeline);
        }
        return false;
    }
    GstBus *bus = gst_element_get_bus(pipeline);
    gst_bus_set_sync_handler(bus, logProblems, nullptr, nullptr);
    gst_object_unref(bus);
    if (gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) {
        __android_log_print(ANDROID_LOG_WARN, kTag, "pipeline %s would not play", launch.c_str());
        gst_object_unref(pipeline);
        return false;
    }
    server->pipeline = pipeline;
    return true;
}

bool startRtsp(Server *server, int port)
{
    server->context = g_main_context_new();
    GstRTSPServer *rtsp = gst_rtsp_server_new();
    const std::string service = std::to_string(port);
    gst_rtsp_server_set_service(rtsp, service.c_str());
    GstRTSPMountPoints *mounts = gst_rtsp_server_get_mount_points(rtsp);
    GstRTSPMediaFactory *factory = gst_rtsp_media_factory_new();
    const std::string launch = std::string("( ") + kTestSource + " ! " + kH264Encoder + " ! rtph264pay name=pay0 pt=96 )";
    gst_rtsp_media_factory_set_launch(factory, launch.c_str());
    gst_rtsp_media_factory_set_shared(factory, TRUE);
    gst_rtsp_mount_points_add_factory(mounts, "/test", factory);
    g_object_unref(mounts);
    if (gst_rtsp_server_attach(rtsp, server->context) == 0) {
        __android_log_print(ANDROID_LOG_WARN, kTag, "RTSP server would not attach on port %d", port);
        gst_object_unref(rtsp);
        g_main_context_unref(server->context);
        server->context = nullptr;
        return false;
    }
    server->rtsp = rtsp;
    server->loop = g_main_loop_new(server->context, FALSE);
    server->thread = std::thread([server]() {
        g_main_context_push_thread_default(server->context);
        g_main_loop_run(server->loop);
        g_main_context_pop_thread_default(server->context);
    });
    return true;
}

void stop(Server *server)
{
    if (server->pipeline) {
        gst_element_set_state(server->pipeline, GST_STATE_NULL);
        gst_object_unref(server->pipeline);
    }
    if (server->loop) {
        g_main_loop_quit(server->loop);
    }
    if (server->thread.joinable()) {
        server->thread.join();
    }
    if (server->loop) {
        g_main_loop_unref(server->loop);
    }
    if (server->rtsp) {
        gst_object_unref(server->rtsp);
    }
    if (server->context) {
        g_main_context_unref(server->context);
    }
    delete server;
}

bool start(Server *server, int type, int port)
{
    const std::string where = std::string(kHost) + ":" + std::to_string(port);
    const std::string sink = std::string("host=") + kHost + " port=" + std::to_string(port);
    const std::string source = kTestSource;
    switch (type) {
    case RtpUdpH264:
        server->uri = "udp://" + where;
        return startPipeline(server, source + " ! " + kH264Encoder + " ! rtph264pay config-interval=1 pt=96 ! udpsink " + sink);
    case RtpUdpH265:
        server->uri = "udp265://" + where;
        return startPipeline(server, source + " ! " + kH265Encoder + " ! rtph265pay config-interval=1 pt=96 ! udpsink " + sink);
    case RtspH264:
        server->uri = "rtsp://" + where + "/test";
        return startRtsp(server, port);
    case MpegTsUdp:
        server->uri = "mpegts://" + where;
        return startPipeline(server, source + " ! " + kH264Encoder + " ! mpegtsmux ! udpsink " + sink);
    case MpegTsTcp:
        server->uri = "tcp://" + where;
        return startPipeline(server, source + " ! " + kH264Encoder + " ! mpegtsmux ! tcpserversink " + sink);
    default:
        return false;
    }
}

}

extern "C" {

__attribute__((visibility("default"))) void *qgc_video_mock_serve(int type, int port)
{
    if (!gst_is_initialized()) {
        __android_log_print(ANDROID_LOG_WARN, kTag, "GStreamer is not initialised");
        return nullptr;
    }
    auto *server = new Server();
    if (!start(server, type, port)) {
        stop(server);
        return nullptr;
    }
    __android_log_print(ANDROID_LOG_INFO, kTag, "serving %s", server->uri.c_str());
    return server;
}

__attribute__((visibility("default"))) const char *qgc_video_mock_uri(void *server)
{
    return server ? static_cast<Server *>(server)->uri.c_str() : "";
}

__attribute__((visibility("default"))) void qgc_video_mock_stop(void *server)
{
    if (server) {
        stop(static_cast<Server *>(server));
    }
}

}
