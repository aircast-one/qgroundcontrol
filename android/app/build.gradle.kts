plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    testOptions {
        unitTests.isReturnDefaultValues = true
    }

    namespace = "one.aircast.android"
    compileSdk = 34

    defaultConfig {
        applicationId = "one.aircast.android"
        minSdk = 28
        targetSdk = 34
        versionCode = 1
        versionName = "0.1"
        ndk { abiFilters += "arm64-v8a" }
    }

    buildFeatures { compose = true }
    packaging { jniLibs { useLegacyPackaging = true } }
    androidResources { noCompress += "rcc" }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    implementation(files("../../qgroundcontrol/build-android/android-build/AircastQGC.aar"))
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
