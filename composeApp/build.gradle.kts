import org.jetbrains.compose.desktop.application.dsl.TargetFormat

val currentOs = System.getProperty("os.name").lowercase()
val currentArch = System.getProperty("os.arch").lowercase()
val javafxPlatform = when {
    currentOs.contains("mac") && (currentArch.contains("aarch64") || currentArch.contains("arm64")) -> "mac-aarch64"
    currentOs.contains("mac") -> "mac"
    currentOs.contains("win") && (currentArch.contains("aarch64") || currentArch.contains("arm64")) -> "win-aarch64"
    currentOs.contains("win") -> "win"
    currentArch.contains("aarch64") || currentArch.contains("arm64") -> "linux-aarch64"
    else -> "linux"
}

plugins {
    kotlin("multiplatform")
    id("org.jetbrains.compose")
    id("org.jetbrains.kotlin.plugin.compose")
}

kotlin {
    jvm("desktop")

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation(compose.runtime)
                implementation(compose.foundation)
                implementation(compose.material3)
                implementation(compose.ui)
                implementation(compose.components.resources)
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
            }
        }

        val commonTest by getting {
            dependencies {
                implementation(kotlin("test"))
            }
        }

        val desktopMain by getting {
            dependencies {
                implementation(compose.desktop.currentOs)
                implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
                implementation("org.openjfx:javafx-base:21.0.9:$javafxPlatform")
                implementation("org.openjfx:javafx-graphics:21.0.9:$javafxPlatform")
                implementation("org.openjfx:javafx-controls:21.0.9:$javafxPlatform")
                implementation("org.openjfx:javafx-media:21.0.9:$javafxPlatform")
                implementation("org.openjfx:javafx-web:21.0.9:$javafxPlatform")
                implementation("org.openjfx:javafx-swing:21.0.9:$javafxPlatform")
            }
        }
    }
}

compose.desktop {
    application {
        mainClass = "com.malinatrash.camundasupport.MainKt"
        javaHome = System.getProperty("java.home")
        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Exe)
            modules(
                "java.instrument",
                "java.management",
                "java.net.http",
                "java.prefs",
                "jdk.jfr",
                "jdk.jsobject",
                "jdk.unsupported",
                "jdk.unsupported.desktop",
                "jdk.xml.dom",
            )
            packageName = "Camunda Support"
            packageVersion = "1.0.0"
            description = "Desktop console for Camunda support operations"
            vendor = "malinatrash"
            macOS {
                bundleID = "com.malinatrash.camundasupport"
            }
        }
    }
}
