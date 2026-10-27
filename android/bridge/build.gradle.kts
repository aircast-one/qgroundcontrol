plugins {
    id("com.android.library")
}

fun qgc(key: String): String = extra["qgc.$key"] as String?
    ?: error("android/qgc.properties has no $key. Run `just android` to generate it.")

val bridgeSources = layout.buildDirectory.dir("bridge")

val copyBridge by tasks.registering(Copy::class) {
    from(rootProject.file("../deploy/android/src")) { include("org/mavlink/qgroundcontrol/QGCBridge.java") }
    into(bridgeSources)
}

android {
    namespace = "org.mavlink.qgroundcontrol.bridge"
    compileSdk = qgc("compileSdk").toInt()

    defaultConfig {
        minSdk = qgc("minSdk").toInt()
    }

    sourceSets {
        getByName("main") {
            java.srcDir(bridgeSources)
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
        targetCompatibility = JavaVersion.toVersion(qgc("javaVersion"))
    }
}

tasks.named("preBuild") { dependsOn(copyBridge) }
