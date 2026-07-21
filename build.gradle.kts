plugins {
    kotlin("multiplatform") version "2.4.10" apply false
    id("org.jetbrains.compose") version "1.11.1" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.4.10" apply false
}

val explicitAppVersion = providers.gradleProperty("appVersion").orNull
val gitDescription = providers.exec {
    commandLine("git", "describe", "--tags", "--match", "v[0-9]*", "--long", "--always", "--dirty")
    isIgnoreExitValue = true
}.standardOutput.asText.get().trim()

val appVersion = explicitAppVersion ?: run {
    val match = Regex("""^v(\d+\.\d+\.\d+)-(\d+)-g([0-9a-f]+)(-dirty)?${'$'}""")
        .matchEntire(gitDescription)
    if (match == null) {
        "0.1.0-dev.0+local"
    } else {
        val (tagVersion, distance, sha, dirty) = match.destructured
        if (distance == "0" && dirty.isEmpty()) tagVersion
        else "$tagVersion-dev.$distance+$sha${if (dirty.isNotEmpty()) ".dirty" else ""}"
    }
}

allprojects {
    group = "com.malinatrash"
    version = appVersion
}
