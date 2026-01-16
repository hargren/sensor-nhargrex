plugins {
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.jetbrainsKotlinAndroid)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.google.devtools.ksp)
    alias(libs.plugins.googleServices)
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_1_8)
    }
}

android {
    namespace = "com.nhargrex.sensor"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.nhargrex.sensor"
        minSdk = 34
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        compose = true
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}

dependencies {
    // Core Android & Compose Dependencies
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom)) // Use the BoM from your TOML
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3) // KEEP: This uses the version from the BoM
    implementation(libs.androidx.compose.material.icons.extended)

    // Firebase Dependencies
    implementation(platform(libs.firebase.bom)) // Import the Firebase BoM
    implementation(libs.firebase.storage)
    implementation(libs.firebase.firestore)
    implementation(libs.firebase.messaging)
    implementation(libs.firebase.ui.auth)

    // Room Database
    implementation(libs.androidx.room.runtime)
    implementation(libs.androidx.room.ktx)
    ksp(libs.androidx.room.compiler)
    //implementation(libs.androidx.compose.remote.creation.core)

    // Media Player
    implementation(libs.androidx.media3.exoplayer)
    implementation(libs.androidx.media3.ui)
    implementation(libs.androidx.media3.session)

    // Other Utility Libraries
    implementation(libs.jwtdecode)
    implementation(libs.androidx.appcompat)

    // Glance App Widget
    implementation(libs.androidx.glance.appwidget)
    // REMOVED: implementation(libs.androidx.compose.material3) was a duplicate

    // Vico Charting Library
    implementation(libs.vico.core)
    implementation(libs.vico.compose)
    implementation(libs.vico.compose.m3)

    // Testing Dependencies
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.ui.test.junit4)
    debugImplementation(libs.androidx.ui.tooling)
    debugImplementation(libs.androidx.ui.test.manifest)

    // FORCE core-ktx to 1.15.0 to stay compatible with AGP 8.7.3
    implementation(libs.androidx.core.ktx) {
        version {
            strictly("1.15.0")
        }
    }

    // Also force the base core library just in case
    implementation("androidx.core:core") {
        version {
            strictly("1.15.0")
        }
    }

    // Force Kotlin Stdlib to match your compiler version (2.1.0)
    constraints {
        implementation("org.jetbrains.kotlin:kotlin-stdlib") {
            version {
                strictly("2.1.0")
            }
        }
        implementation("org.jetbrains.kotlin:kotlin-stdlib-jdk8") {
            version {
                strictly("2.1.0")
            }
        }
    }
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    compilerOptions {
        // This forces the stable 1.9 engine to avoid the K2 analysis crash
        languageVersion.set(org.jetbrains.kotlin.gradle.dsl.KotlinVersion.KOTLIN_1_9)
    }
}

