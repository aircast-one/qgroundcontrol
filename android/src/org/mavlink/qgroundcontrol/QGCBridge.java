package org.mavlink.qgroundcontrol;

import android.os.Handler;
import android.os.Looper;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;

public final class QGCBridge {
    public interface Host {
        void setSystemBarAppearance(boolean lightBars);
    }

    public interface EventListener {
        void onEvent(String path, String json);
    }

    private static final String DEFAULT_CLIENT = "default";

    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static volatile Host host = null;
    private static final Map<String, EventListener> listeners = new LinkedHashMap<>();
    private static final Map<String, String> watchedByClient = new LinkedHashMap<>();

    private QGCBridge() {
    }

    public static void setHost(final Host newHost) {
        host = newHost;
    }

    public static void setEventListener(final EventListener listener) {
        setEventListener(DEFAULT_CLIENT, listener);
    }

    public static synchronized void setEventListener(final String client, final EventListener listener) {
        if (listener == null) {
            listeners.remove(client);
        } else {
            listeners.put(client, listener);
        }
    }

    public static void watch(final String paths) {
        watch(DEFAULT_CLIENT, paths);
    }

    public static synchronized void watch(final String client, final String paths) {
        watchedByClient.put(client, paths == null ? "" : paths);

        final LinkedHashSet<String> union = new LinkedHashSet<>();
        for (final String set : watchedByClient.values()) {
            for (final String path : set.split(",")) {
                final String trimmed = path.trim();
                if (!trimmed.isEmpty()) {
                    union.add(trimmed);
                }
            }
        }
        nativeWatch(String.join(",", union));
    }

    public static native String get(String path);

    public static native String set(String path, String json);

    public static native String invoke(String path, String jsonArgs);

    private static native void nativeWatch(String paths);

    public static native void notifyFontScale(float scale);

    public static native void notifySafeAreaInsets(int left, int top, int right, int bottom);

    public static native void notifyDeepLink(String url);

    public static native int videoWidth();

    public static native int videoHeight();

    public static native long videoFrames();

    /** dest must be a direct ByteBuffer; returns false when no frame has arrived yet. */
    public static native boolean videoCopyFrame(java.nio.ByteBuffer dest);

    public static native boolean videoSetSurface(android.view.Surface surface);

    public static void onSystemBarAppearance(final boolean lightBars) {
        final Host current = host;
        if (current == null) {
            return;
        }
        MAIN.post(new Runnable() {
            @Override
            public void run() {
                current.setSystemBarAppearance(lightBars);
            }
        });
    }

    public static void onEvent(final String path, final String json) {
        final List<EventListener> current;
        synchronized (QGCBridge.class) {
            if (listeners.isEmpty()) {
                return;
            }
            current = new ArrayList<>(listeners.values());
        }
        MAIN.post(new Runnable() {
            @Override
            public void run() {
                for (final EventListener listener : current) {
                    listener.onEvent(path, json);
                }
            }
        });
    }
}
