#include "../Core/GStreamerPlugins.h"

#ifdef QGC_GST_STREAMING
#include <gst/gst.h>

#define QGC_PLUGINS(X) \
    X(coreelements) X(app) X(typefindfunctions) X(playback) X(videoconvertscale) \
    X(isomp4) X(matroska) X(mpegtsdemux) X(multifile) X(libav) X(openh264) X(vpx) \
    X(rtp) X(rtpmanager) X(rtsp) X(sdpelem) X(tcp) X(udp) X(videoparsersbad) \
    X(applemedia) X(opengl) X(webrtc) X(webrtchttp) X(nice) X(dtls) X(srtp) X(sctp)

#define QGC_DECLARE(name) GST_PLUGIN_STATIC_DECLARE(name);
QGC_PLUGINS(QGC_DECLARE)

extern void g_io_openssl_load(gpointer module);
#endif

void qgc_ios_register_gstreamer_plugins(const char *ca_certificates)
{
#ifdef QGC_GST_STREAMING
    static gboolean registered = FALSE;
    if (registered) {
        return;
    }
    registered = TRUE;
    if (ca_certificates) {
        g_setenv("CA_CERTIFICATES", ca_certificates, TRUE);
    }
    gst_init(NULL, NULL);
    g_io_openssl_load(NULL);
#define QGC_REGISTER(name) GST_PLUGIN_STATIC_REGISTER(name);
    QGC_PLUGINS(QGC_REGISTER)
#else
    (void)ca_certificates;
#endif
}
