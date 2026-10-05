plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

fun qgc(key: String): String = extra["qgc.$key"] as String?
    ?: error(
        "android/qgc.properties has no $key. Run `just android` to generate it" +
            (extra.properties["qgc.$key.env"]?.let { ", or set $it" } ?: "") + "."
    )

android {
    namespace = "one.aircast.map"
    compileSdk = qgc("compileSdk").toInt()

    defaultConfig {
        minSdk = qgc("minSdk").toInt()
    }

    buildFeatures { compose = true }

    compileOptions {
        sourceCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
        targetCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
    }
    kotlinOptions { jvmTarget = qgc("javaVersion") }
}

dependencies {
    compileOnly(files(qgc("aar")))
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2024.09.03"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("org.maplibre.gl:android-sdk:11.13.5")
    compileOnly("com.squareup.okhttp3:okhttp:4.12.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
}
