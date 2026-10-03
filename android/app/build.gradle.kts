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

val coreCrate = rootProject.file("../groundstation")
val coreJniLibs = layout.buildDirectory.dir("core/jniLibs")
val coreBridgeSources = layout.buildDirectory.dir("core/bridge")
val instrumentIconAssets = layout.buildDirectory.dir("instrumentIcons")
val coreVideoBuild = layout.buildDirectory.dir("core/video/${qgc("abi")}")
val coreTriples = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64" to "x86_64-linux-android",
    "x86" to "i686-linux-android",
)

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

    flavorDimensions += "host"
    productFlavors {
        create("qt") { dimension = "host" }
        create("core") { dimension = "host"; applicationIdSuffix = ".core" }
    }
    sourceSets {
        getByName("main") {
            assets.srcDir(instrumentIconAssets)
        }
        getByName("core") {
            java.srcDir(coreBridgeSources)
            java.srcDir(coreVideoBuild.map { it.dir("android-build/src") })
            jniLibs.srcDir(coreJniLibs)
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

val copyInstrumentIcons by tasks.registering(Copy::class) {
    from(rootProject.file("../resources/InstrumentValueIcons"))
    into(instrumentIconAssets.map { it.dir("InstrumentValueIcons") })
}

val copyAirframeImages by tasks.registering(Copy::class) {
    from(rootProject.file("../src/AutoPilotPlugins/Common/Images")) { include("*.svg") }
    into(instrumentIconAssets.map { it.dir("Airframe") })
}

val copySectionImages by tasks.registering(Copy::class) {
    from(rootProject.file("../src/Toolbar/Images")) { include("Battery.svg", "Gears.svg") }
    from(rootProject.file("../src/AutoPilotPlugins/PX4/Images")) {
        include(
            "DatalinkLoss.svg", "GeoFence.svg", "LandModeCopter.svg", "LowBattery.svg", "ObjectAvoidance.svg",
            "RCLoss.svg", "ReturnToHomeAltitude.svg", "ReturnToHomeAltitudeCopter.svg",
        )
    }
    into(instrumentIconAssets.map { it.dir("SetupSections") })
}

val copyCoreBridge by tasks.registering(Copy::class) {
    from(rootProject.file("../deploy/android/src")) {
        include(listOf("QGCBridge", "QGCUsbSerialManager", "QGCUsbSerialProber", "QGCUsbId", "QGCFtdiDriver", "QGCFtdiSerialDriver", "QGCLogger").map { "org/mavlink/qgroundcontrol/$it.java" })
    }
    into(coreBridgeSources)
}

val buildCoreLibrary by tasks.registering(Exec::class) {
    val abi = qgc("abi")
    val triple = coreTriples[abi] ?: error("no Rust target for the $abi ABI")
    workingDir = coreCrate
    environment("ANDROID_NDK_HOME", android.ndkDirectory.absolutePath)
    commandLine("cargo", "ndk", "--target", abi, "--platform", qgc("minSdk"), "rustc", "--lib", "--release", "--features", "jni-host", "--crate-type", "cdylib")
    doLast {
        copy {
            from(coreCrate.resolve("target/$triple/release/libgroundstation.so"))
            into(coreJniLibs.get().dir(abi))
        }
    }
}

val buildCoreVideo by tasks.registering(Exec::class) {
    val abi = qgc("abi")
    val source = rootProject.file("video")
    val build = coreVideoBuild.get().asFile
    val toolchain = android.ndkDirectory.resolve("build/cmake/android.toolchain.cmake")
    commandLine(
        "sh", "-c",
        "cmake -S \"$source\" -B \"$build\" -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=\"$toolchain\" " +
            "-DANDROID_ABI=$abi -DANDROID_PLATFORM=${qgc("minSdk")} && cmake --build \"$build\"",
    )
    doLast {
        copy {
            from(build.resolve("libqgc_video.so"), build.resolve("libqgc_wfb.so"))
            into(coreJniLibs.get().dir(abi))
        }
    }
}

tasks.named("preBuild") { dependsOn(copyInstrumentIcons, copyAirframeImages, copySectionImages) }

tasks.matching { it.name.startsWith("preCore") && it.name.endsWith("Build") }.configureEach {
    dependsOn(copyCoreBridge, buildCoreLibrary, buildCoreVideo)
}

dependencies {
    "qtImplementation"(files(qgc("aar")))
    "coreImplementation"(files(rootProject.file("../deploy/android/libs/d2xx.jar")))
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("com.github.mik3y:usb-serial-for-android:3.10.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2024.09.03"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation(project(":map-spike"))
    implementation("androidx.compose.material:material-icons-core")
    implementation("io.coil-kt.coil3:coil-compose:3.1.0")
    implementation("io.coil-kt.coil3:coil-svg:3.1.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
    testImplementation("org.jetbrains.kotlin:kotlin-metadata-jvm:2.0.21")
}
