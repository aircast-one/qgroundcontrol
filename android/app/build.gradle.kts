plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.github.triplet.play")
}

play {
    serviceAccountCredentials.set(file(System.getenv("AIRCAST_PLAY_SERVICE_ACCOUNT") ?: "${System.getProperty("user.home")}/.config/aircast/play-service-account.json"))
    track.set(System.getenv("AIRCAST_PLAY_TRACK") ?: "internal")
    defaultToAppBundles.set(true)
}

android {
    testOptions {
        unitTests.isReturnDefaultValues = true
    }

    namespace = "one.aircast.android"
    compileSdk = 36

    defaultConfig {
        applicationId = "one.aircast.app"
        minSdk = 28
        targetSdk = 36
        versionCode = (System.getenv("AIRCAST_VERSION_CODE") ?: "1").toInt()
        versionName = System.getenv("AIRCAST_VERSION_NAME") ?: "0.1"
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        create("release") {
            val store = System.getenv("AIRCAST_KEYSTORE")
            if (store != null) {
                storeFile = file(store)
                storePassword = System.getenv("AIRCAST_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("AIRCAST_KEY_ALIAS")
                keyPassword = System.getenv("AIRCAST_KEY_PASSWORD") ?: System.getenv("AIRCAST_KEYSTORE_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("release").takeIf { it.storeFile != null }
        }
    }

    buildFeatures { compose = true; buildConfig = true }
    packaging { jniLibs { useLegacyPackaging = true } }
    androidResources { noCompress += "rcc" }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    implementation(files(System.getenv("AIRCAST_QGC_AAR") ?: "../../build-android/android-build/AircastQGC.aar"))
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("com.github.mik3y:usb-serial-for-android:3.8.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2024.09.03"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation(project(":map-spike"))
    implementation("androidx.compose.material:material-icons-core")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
}
