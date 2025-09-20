import org.gradle.api.tasks.Exec

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidKotlinMultiplatformLibrary)
}


// TODO: Make target dir dynamic based on OS
val rustTargetDir = "target/x86_64-pc-windows-msvc/release"
val rustDllName = "mihon_runner.dll"
val jniLibDir = "/jni_libraries"

tasks.register<Exec>("buildAndCopyRust") {
    // Step 1: Build Rust
    workingDir = file("src/commonMain/rust")
    commandLine("cargo", "build", "--release", "--target", "x86_64-pc-windows-msvc")

    // Step 2: After Rust is built, copy the DLL
    doLast {
        val userHomePath = System.getProperty("user.home")
        val sourceDll = file("$rustTargetDir/$rustDllName")
        val targetDir = file("$userHomePath$jniLibDir")
        targetDir.mkdirs()
        sourceDll.copyTo(file(targetDir.resolve(rustDllName)), overwrite = true)
        println("Copied Rust library to $jniLibDir")
    }
}

kotlin {
    jvm()
    androidLibrary {
        namespace = "mihonx.runner"
        compileSdk = 36
        minSdk = 24
    }

    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "mihonx-runnerKit"
        }
    }

    sourceSets {
        commonMain {
            dependencies {
                implementation(libs.kotlin.stdlib)
                // Add KMP dependencies here
            }
        }

        androidMain {
            dependencies { }
        }

        iosMain {
            dependencies { }
        }

        jvmMain {
            dependencies {

            }
        }
    }
}