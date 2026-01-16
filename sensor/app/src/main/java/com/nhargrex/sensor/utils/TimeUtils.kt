package com.nhargrex.sensor.utils

fun timeAgo(timestamp: Double): String {
    val now = System.currentTimeMillis() / 1000.0
    val diff = now - timestamp

    return when {
        diff < 60 -> "Updated just now"
        diff < 5 * 60 -> "Updated less than 5 minutes ago"
        diff < 60 * 60 -> "Updated ${(diff / 60).toInt()} minutes ago"
        diff < 2 * 60 * 60 -> "Updated about an hour ago"
        else -> "Updated more than an hour ago"
    }
}