package com.nhargrex.sensor

import android.annotation.SuppressLint
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.util.Log
import android.widget.RemoteViews
import androidx.core.app.NotificationCompat
import com.google.firebase.messaging.Constants.MessageNotificationKeys.TAG
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

const val channelId = "notification_channel"
const val channelName = "com.nhargrex.sensor"

@SuppressLint("MissingFirebaseInstanceTokenRefresh")
class FCMService : FirebaseMessagingService() {

    override fun onMessageReceived(message: RemoteMessage) {
        // Kotlin 2.1.0 might crash if you use RemoteMessage? (nullable)
        // or if there's a syntax error on line 21
        super.onMessageReceived(message)
    }

    override fun onNewToken(token: String) {
        super.onNewToken(token)
    }
}
/*
class FCMService : FirebaseMessagingService() {

    // Ensure this signature is exactly as follows:
    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        // Use safe calls or let to handle the potential nulls inside the message
        remoteMessage.notification?.let { notification ->
            generateNotification(notification.title ?: "", notification.body ?: "")
        }
    }

    private fun getRemoteView(title: String, message: String) : RemoteViews {
        val remoteView = RemoteViews("com.nhargrex.sensor", R.layout.notification)
        remoteView.setTextViewText(R.id.title, title)
        remoteView.setTextViewText(R.id.description, message)
        remoteView.setImageViewResource(R.id.imageView, R.drawable.logo)
        return remoteView
    }

    private fun generateNotification(title: String, message: String) {

        val intent = Intent(this, MainActivity::class.java)

        intent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)

        var builder: NotificationCompat.Builder = NotificationCompat.Builder(applicationContext, channelId)
            .setSmallIcon(R.drawable.logo)
            .setAutoCancel(true)
            .setVibrate(longArrayOf(1000, 1000, 1000, 1000))
            .setOnlyAlertOnce(true)
            .setContentIntent(PendingIntent.getActivity(this, 0, intent, PendingIntent.FLAG_ONE_SHOT or PendingIntent.FLAG_IMMUTABLE))

        Log.d(TAG, "Title: $title Message: $message")

        builder = builder.setContent(getRemoteView(title, message))

        val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager

        notificationManager.createNotificationChannel(NotificationChannel(channelId, channelName, NotificationManager.IMPORTANCE_HIGH))

        notificationManager.notify(0, builder.build())
    }
}
*/
