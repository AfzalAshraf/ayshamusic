plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Audio-bridge instrumentation deliberately excludes JNI/provider acquisition.
// Release tasks refuse this mode; production always packages ARM64 Rust.
val emulatorTests = providers.gradleProperty("ayshaEmulatorTests").orNull == "true"
check(!emulatorTests || gradle.startParameter.taskNames.none { it.contains("Release", ignoreCase = true) })

android {
    namespace = "com.afzal.ayshamusic"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.afzal.ayshamusic"
        // 26: cpal's AAudio output backend requires libaaudio (added in API 26).
        minSdk = 26
        targetSdk = 35
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        versionCode = 24
        versionName = "2.2.0"
        // CRITICAL: Only arm64-v8a for minimal APK size
        ndk {
            abiFilters += if (emulatorTests) "x86_64" else "arm64-v8a"
        }

        // CRITICAL: extractNativeLibs=false -> mmap .so directly from the APK
        packagingOptions {
            jniLibs {
                useLegacyPackaging = false
            }
        }
    }

    if (emulatorTests) {
        sourceSets.getByName("main").jniLibs.setSrcDirs(emptyList<String>())
    }

    // Must be declared before buildTypes so the release type can reference it.
    // When KEYSTORE_PATH is unset/missing, the config stays empty and the release
    // only explicitly labelled preview builds may use debug signing.
    signingConfigs {
        create("release") {
            val keystorePath = System.getenv("KEYSTORE_PATH")
            if (!keystorePath.isNullOrBlank() && file(keystorePath).exists()) {
                storeFile = file(keystorePath)
                storePassword = System.getenv("KEYSTORE_PASSWORD").orEmpty()
                keyAlias = System.getenv("KEY_ALIAS").orEmpty()
                keyPassword = System.getenv("KEY_PASSWORD").orEmpty()
            }
        }
    }

    flavorDimensions += "audio"
    productFlavors {
        create("androidAudio") {
            dimension = "audio"
            applicationIdSuffix = ".androidaudio10"
            resValue("string", "app_name", "Aysha Music")
            buildConfigField("String", "AUDIO_ENGINE", "\"media3\"")
            buildConfigField("int", "API_PORT", "17652")
        }
    }

    buildTypes {
        release {
            // CI test builds install alongside the old APK: ephemeral debug keys
            // must not force the user to uninstall it and lose local preferences.
            if (System.getenv("AYSHA_PREVIEW") == "1") {
                versionNameSuffix = "-preview"
            }
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            val releaseSigning = signingConfigs.getByName("release")
            signingConfig = if (releaseSigning.storeFile != null) {
                releaseSigning
            } else {
                check(System.getenv("AYSHA_PREVIEW") == "1" || emulatorTests || gradle.startParameter.taskNames.none { it.contains("Release", ignoreCase = true) }) {
                    "Production release requires your release keystore. Use AYSHA_PREVIEW=1 only for explicitly labelled test APKs."
                }
                // Preview/debug test fallback only; never silently sign production with a debug key.
                // Provide KEYSTORE_PATH/KEYSTORE_PASSWORD/KEY_ALIAS/KEY_PASSWORD
                // for a production-signed APK.
                signingConfigs.getByName("debug")
            }
        }
        debug {
            isMinifyEnabled = false
            isShrinkResources = false
            isDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }

    // No ViewBinding, no DataBinding - keep it minimal
    buildFeatures {
        buildConfig = true
        viewBinding = false
        dataBinding = false
    }
}

dependencies {
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    // Minimal AndroidX - only what we absolutely need
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.media:media:1.6.0")  // MediaSessionCompat

    "androidAudioImplementation"("androidx.media3:media3-exoplayer:1.8.0")

    // NO Jetpack Compose, NO Navigation, NO Room, NO WorkManager, NO Lifecycle, NO ConstraintLayout
}