package com.nhargrex.sensor

import android.app.Application
import androidx.room.Room

class SensorApp : Application() {

    lateinit var database: AppDatabase
        private set

    override fun onCreate() {
        super.onCreate()

        database = Room.databaseBuilder(
            applicationContext,
            AppDatabase::class.java,
            "sensor-db"
        ).build()
    }
}