package org.mavlink.qgroundcontrol;

import java.io.File;
import java.util.List;
import java.lang.reflect.Method;

import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.PowerManager;
import android.net.wifi.WifiManager;
import android.provider.Settings;
import android.util.Log;
import android.view.View;
import android.view.WindowManager;
import android.app.Activity;
import android.os.storage.StorageManager;
import android.os.storage.StorageVolume;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;

import org.qtproject.qt.android.bindings.QtActivity;

public class QGCActivity extends QtActivity {
    private static final String TAG = QGCActivity.class.getSimpleName();
    private static final String SCREEN_BRIGHT_WAKE_LOCK_TAG = "QGroundControl";
    private static final String MULTICAST_LOCK_TAG = "QGroundControl";

    private static QGCActivity m_instance = null;

    private PowerManager.WakeLock m_wakeLock;
    private WifiManager.MulticastLock m_wifiMulticastLock;

    public QGCActivity() {
        m_instance = this;
    }

    public static QGCActivity getInstance() {
        return m_instance;
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        nativeInit();
        reportWindowInsets();
        acquireWakeLock();
        keepScreenOn();
        setupMulticastLock();

        QGCUsbSerialManager.initialize(this);
    }

    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        nativeFontScaleChanged(newConfig.fontScale);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);

        if (intent == null || !Intent.ACTION_VIEW.equals(intent.getAction())) {
            return;
        }

        final Uri data = intent.getData();
        if (data != null) {
            nativeDeepLink(data.toString());
        }
    }

    @Override
    protected void onDestroy() {
        try {
            releaseMulticastLock();
            releaseWakeLock();
            QGCUsbSerialManager.cleanup(this);
        } catch (final Exception e) {
            Log.e(TAG, "Exception onDestroy()", e);
        }

        super.onDestroy();
    }

    public static void setSystemBarAppearance(boolean lightBars) {
        final Activity activity = m_instance;
        if (activity == null) {
            return;
        }

        activity.runOnUiThread(() -> {
            final WindowInsetsControllerCompat controller =
                WindowCompat.getInsetsController(activity.getWindow(), activity.getWindow().getDecorView());
            controller.setAppearanceLightStatusBars(lightBars);
            controller.setAppearanceLightNavigationBars(lightBars);
        });
    }

    private void reportWindowInsets() {
        final View decor = getWindow().getDecorView();
        ViewCompat.setOnApplyWindowInsetsListener(decor, (view, windowInsets) -> {
            final Insets insets = windowInsets.getInsets(
                WindowInsetsCompat.Type.systemBars() | WindowInsetsCompat.Type.displayCutout());
            nativeSafeAreaInsets(insets.left, insets.top, insets.right, insets.bottom);
            return windowInsets;
        });
        ViewCompat.requestApplyInsets(decor);
    }

    private void keepScreenOn() {
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
    }

    private void acquireWakeLock() {
        final PowerManager pm = (PowerManager) getSystemService(Context.POWER_SERVICE);
        m_wakeLock = pm.newWakeLock(PowerManager.SCREEN_BRIGHT_WAKE_LOCK, SCREEN_BRIGHT_WAKE_LOCK_TAG);
        if (m_wakeLock != null) {
            m_wakeLock.acquire();
        } else {
            Log.w(TAG, "SCREEN_BRIGHT_WAKE_LOCK not acquired!");
        }
    }

    private void releaseWakeLock() {
        if (m_wakeLock != null && m_wakeLock.isHeld()) {
            m_wakeLock.release();
        }
    }

    private void setupMulticastLock() {
        if (m_wifiMulticastLock == null) {
            final WifiManager wifi = (WifiManager) getSystemService(Context.WIFI_SERVICE);
            m_wifiMulticastLock = wifi.createMulticastLock(MULTICAST_LOCK_TAG);
            m_wifiMulticastLock.setReferenceCounted(true);
        }

        m_wifiMulticastLock.acquire();
        Log.d(TAG, "Multicast lock: " + m_wifiMulticastLock.toString());
    }

    private void releaseMulticastLock() {
        if (m_wifiMulticastLock != null && m_wifiMulticastLock.isHeld()) {
            m_wifiMulticastLock.release();
            Log.d(TAG, "Multicast lock released.");
        }
    }

    public static String getSDCardPath() {
        StorageManager storageManager = (StorageManager)m_instance.getSystemService(Activity.STORAGE_SERVICE);
        List<StorageVolume> volumes = storageManager.getStorageVolumes();
        
        for (StorageVolume vol : volumes) {
            if (!vol.isRemovable()) {
                continue;
            }
            
            String path = null;
            
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                File directory = vol.getDirectory();
                if (directory != null) {
                    path = directory.getAbsolutePath();
                }
            } else {
                try {
                    Method mMethodGetPath = vol.getClass().getMethod("getPath");
                    path = (String) mMethodGetPath.invoke(vol);
                } catch (Exception e) {
                    Log.e(TAG, "Failed to get path via reflection", e);
                    continue;
                }
            }
            
            if (path != null && !path.isEmpty()) {
                Log.i(TAG, "removable sd card mounted at " + path);
                return path;
            }
        }
        
        Log.w(TAG, "No removable SD card found");
        return "";
    }

    public static boolean checkStoragePermissions() {
        if (m_instance == null) {
            Log.e(TAG, "Activity instance is null");
            return false;
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            if (!Environment.isExternalStorageManager()) {
                Log.i(TAG, "MANAGE_EXTERNAL_STORAGE not granted, requesting...");
                try {
                    Intent intent = new Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION);
                    intent.setData(Uri.parse("package:" + m_instance.getPackageName()));
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    m_instance.startActivity(intent);
                } catch (Exception e) {
                    Log.e(TAG, "Failed to open storage permission settings", e);
                    Intent intent = new Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION);
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    m_instance.startActivity(intent);
                }
                return false;
            }
            Log.i(TAG, "MANAGE_EXTERNAL_STORAGE already granted");
            return true;
        } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            String[] permissions = {
                android.Manifest.permission.READ_EXTERNAL_STORAGE,
                android.Manifest.permission.WRITE_EXTERNAL_STORAGE
            };

            boolean allGranted = true;
            for (String permission : permissions) {
                if (ContextCompat.checkSelfPermission(m_instance, permission) != PackageManager.PERMISSION_GRANTED) {
                    allGranted = false;
                    break;
                }
            }

            if (!allGranted) {
                Log.i(TAG, "Storage permissions not granted, requesting...");
                ActivityCompat.requestPermissions(m_instance, permissions, 1);
                return false;
            }

            Log.i(TAG, "Storage permissions already granted");
            return true;
        } else {
            return true;
        }
    }

    public native boolean nativeInit();
    public native void nativeSafeAreaInsets(int left, int top, int right, int bottom);
    public native void nativeDeepLink(final String url);
    public native void nativeFontScaleChanged(final float scale);
    public native void qgcLogDebug(final String message);
    public native void qgcLogWarning(final String message);
}
