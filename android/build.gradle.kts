plugins {
    id("com.android.application") version "8.8.0" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.21" apply false
    id("com.github.triplet.play") version "3.12.1" apply false
}

val fixtureKeys by tasks.registering(Exec::class) {
    description = "Fails when a test fixture key is decoded by the head but emitted by no producer."
    group = "verification"
    commandLine("python3", "$rootDir/tools/fixturekeys.py")
}

val readinessKeys by tasks.registering(Exec::class) {
    description = "Fails when the core computes a readiness flag this head never reads."
    group = "verification"
    commandLine("python3", "$rootDir/tools/readinesskeys.py")
}

subprojects {
    plugins.withId("com.android.base") {
        tasks.matching { it.name == "check" || it.name == "test" }.configureEach {
            dependsOn(fixtureKeys, readinessKeys)
        }
    }
}
