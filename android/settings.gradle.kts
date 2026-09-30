pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        maven { url = uri("https://jitpack.io") }
    }
}
rootProject.name = "aircast-android"
include(":app")
include(":map-spike")

val buildConfig = groovy.json.JsonSlurper().parse(file("../.github/build-config.json")) as Map<*, *>
val androidConfig = buildConfig["android"] as Map<*, *>

val qgcFile = file("qgc.properties")
val qgcProperties = java.util.Properties().also {
    if (qgcFile.isFile) qgcFile.inputStream().use(it::load)
}

fun blankToNull(value: String?) = value?.takeIf { it.isNotBlank() }

fun pinned(key: String): String =
    blankToNull(androidConfig[key]?.toString())
        ?: error(".github/build-config.json has no android.$key")

fun generated(key: String, environmentVariable: String): String? =
    blankToNull(System.getenv(environmentVariable)) ?: blankToNull(qgcProperties.getProperty(key))

val qgcSettings = mapOf(
    "compileSdk" to pinned("platform"),
    "targetSdk" to pinned("platform"),
    "minSdk" to pinned("min_sdk"),
    "ndkVersion" to pinned("ndk_full_version"),
    "javaVersion" to pinned("java_version"),
    "aar" to generated("aar", "AIRCAST_QGC_AAR"),
    "abi" to generated("abi", "AIRCAST_ABI"),
    "versionName" to generated("versionName", "AIRCAST_VERSION_NAME"),
    "versionCode" to generated("versionCode", "AIRCAST_VERSION_CODE"),
)

val qgcHints = mapOf(
    "aar" to "AIRCAST_QGC_AAR",
    "abi" to "AIRCAST_ABI",
    "versionName" to "AIRCAST_VERSION_NAME",
    "versionCode" to "AIRCAST_VERSION_CODE",
)

gradle.beforeProject {
    qgcSettings.forEach { (key, value) -> extra["qgc.$key"] = value }
    qgcHints.forEach { (key, variable) -> extra["qgc.$key.env"] = variable }
}
