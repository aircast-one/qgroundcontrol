plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.github.triplet.play")
}

fun qgc(key: String): String = extra["qgc.$key"] as String?
    ?: error(
        "android/qgc.properties has no $key. Run `just android` to generate it" +
            (extra.properties["qgc.$key.env"]?.let { ", or set $it" } ?: "") + "."
    )

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
    compileSdk = qgc("compileSdk").toInt()
    ndkVersion = qgc("ndkVersion")

    defaultConfig {
        applicationId = "one.aircast.app"
        minSdk = qgc("minSdk").toInt()
        targetSdk = qgc("targetSdk").toInt()
        versionCode = qgc("versionCode").toInt()
        versionName = qgc("versionName")
        ndk { abiFilters += qgc("abi") }
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
        sourceCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
        targetCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
    }
    kotlinOptions { jvmTarget = qgc("javaVersion") }
}

dependencies {
    implementation(files(qgc("aar")))
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
