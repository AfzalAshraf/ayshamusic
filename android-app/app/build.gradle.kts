plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.afzal.ayshamusic"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.afzal.ayshamusic"
        // 26: cpal's AAudio output backend requires libaaudio (added in API 26).
        minSdk = 26
        targetSdk = 35
        versionCode = 2
        versionName = "1.0.1"

        // CRITICAL: Only arm64-v8a for minimal APK size
        ndk {
            abiFilters += "arm64-v8a"
        }

        // CRITICAL: extractNativeLibs=false -> mmap .so directly from the APK
        packagingOptions {
            jniLibs {
                useLegacyPackaging = false
            }
        }
    }

    // Must be declared before buildTypes so the release type can reference it.
    // When KEYSTORE_PATH is unset/missing, the config stays empty and the release
    // build falls back to the debug signing config below instead of failing.
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

    buildTypes {
        release {
            // CI test builds install alongside the old APK: ephemeral debug keys
            // must not force the user to uninstall it and lose local preferences.
            if (System.getenv("AYSHA_PREVIEW") == "1") {
                applicationIdSuffix = ".preview"
                versionNameSuffix = "-preview"
                resValue("string", "app_name", "Aysha Music Preview")
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
                // Safe fallback: keeps `assembleRelease` usable without secrets.
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
        viewBinding = false
        dataBinding = false
    }
}

dependencies {
    // Minimal AndroidX - only what we absolutely need
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.media:media:1.6.0")  // MediaSessionCompat

    // NO Jetpack Compose, NO Navigation, NO Room, NO WorkManager, NO Lifecycle, NO ConstraintLayout
}