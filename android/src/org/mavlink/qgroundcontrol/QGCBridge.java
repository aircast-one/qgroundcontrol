package org.mavlink.qgroundcontrol;

import android.os.Handler;
import android.os.Looper;

public final class QGCBridge {
    public interface Host {
        void setSystemBarAppearance(boolean lightBars);
    }

    public interface EventListener {
        void onEvent(String path, String json);
    }

    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static volatile Host host = null;
    private static volatile EventListener eventListener = null;

    private QGCBridge() {
    }

    public static void setHost(final Host newHost) {
        host = newHost;
    }

    public static void setEventListener(final EventListener listener) {
        eventListener = listener;
    }

    public static native String get(String path);

    public static native String set(String path, String json);

    public static native String invoke(String path, String jsonArgs);

    public static native void watch(String paths);

    public static native void notifyFontScale(float scale);

    public static native void notifySafeAreaInsets(int left, int top, int right, int bottom);

    public static native void notifyDeepLink(String url);

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
        final EventListener current = eventListener;
        if (current == null) {
            return;
        }
        MAIN.post(new Runnable() {
            @Override
            public void run() {
                current.onEvent(path, json);
            }
        });
    }
}
