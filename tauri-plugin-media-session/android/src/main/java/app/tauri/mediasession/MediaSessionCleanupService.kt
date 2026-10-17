package app.tauri.mediasession

import android.app.Notification
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log

/**
 * Foreground service that keeps the process alive for the entire duration of a media session.
 *
 * Acquired on session start, released only on session clear:
 * - Foreground service: prevents process kill and network throttling
 * - PARTIAL_WAKE_LOCK: keeps CPU alive so JS can execute between tracks
 * - AudioFocus: prevents other apps (calls, notifications) from interrupting playback;
 *   emits "pause"/"play" to JS on focus loss/gain
 * - AUDIO_BECOMING_NOISY receiver: emits "pause" when headphones are unplugged
 */
class MediaSessionCleanupService : Service() {

    companion object {
        private const val TAG = "plugin/media-session"
        private const val ACTION_INIT = "app.tauri.mediasession.ACTION_INIT"
        internal const val NOTIFICATION_ID = 9401

        @Volatile internal var instance: MediaSessionCleanupService? = null
        @Volatile internal var pendingNotification: Notification? = null

        /**
         * Start (or update) the foreground service with the given notification.
         * Must be called while the app is in the foreground on first call.
         */
        fun start(context: Context, notification: Notification) {
            pendingNotification = notification
            val svc = instance
            if (svc != null) {
                svc.postNotification(notification)
            } else {
                try {
                    context.startForegroundService(
                        Intent(context, MediaSessionCleanupService::class.java)
                            .setAction(ACTION_INIT)
                    )
                } catch (e: Exception) {
                    Log.e(TAG, "startForegroundService failed: ${e.message}")
                }
            }
        }

        /**
         * Stop the foreground service and release all resources.
         * Safe to call from any context — uses the direct instance reference.
         */
        fun stop() {
            instance?.handleStop()
        }
    }

    private var wakeLock: PowerManager.WakeLock? = null
    private var audioFocusRequest: AudioFocusRequest? = null  // API 26+
    private var noisyReceiver: BroadcastReceiver? = null

    // ── Service lifecycle ────────────────────────────────────────────────────

    override fun onCreate() {
        super.onCreate()
        instance = this
        Log.d(TAG, "onCreate")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_INIT) {
            val notification = pendingNotification ?: run {
                Log.w(TAG, "onStartCommand: no notification, stopping")
                stopSelf()
                return START_NOT_STICKY
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    NOTIFICATION_ID, notification,
                    android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
                )
            } else {
                startForeground(NOTIFICATION_ID, notification)
            }
            acquireWakeLock()
            requestAudioFocus()
            registerNoisyReceiver()
            Log.d(TAG, "Foreground started, locks acquired")
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onTaskRemoved(rootIntent: Intent?) {
        Log.d(TAG, "onTaskRemoved")
        instance = null
        releaseResources()
        MediaSessionPlugin.forceCleanup(applicationContext)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
        super.onTaskRemoved(rootIntent)
        android.os.Process.killProcess(android.os.Process.myPid())
    }

    override fun onDestroy() {
        Log.d(TAG, "onDestroy")
        instance = null
        releaseResources()
        MediaSessionPlugin.forceCleanup(applicationContext)
        super.onDestroy()
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    internal fun postNotification(notification: Notification) {
        val nm = getSystemService(NOTIFICATION_SERVICE) as android.app.NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    private fun handleStop() {
        releaseResources()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
    }

    private fun releaseResources() {
        unregisterNoisyReceiver()
        releaseWakeLock()
        abandonAudioFocus()
    }

    // ── WakeLock ─────────────────────────────────────────────────────────────

    private fun acquireWakeLock() {
        if (wakeLock?.isHeld == true) return
        val pm = getSystemService(POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "app.tauri.mediasession:PlaybackWakeLock"
        ).apply { acquire(24 * 60 * 60 * 1000L) }
        Log.d(TAG, "WakeLock acquired")
    }

    private fun releaseWakeLock() {
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
        Log.d(TAG, "WakeLock released")
    }

    // ── AudioFocus ───────────────────────────────────────────────────────────

    private fun requestAudioFocus() {
        val am = getSystemService(AUDIO_SERVICE) as AudioManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            if (audioFocusRequest != null) return
            val req = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build()
                )
                .setAcceptsDelayedFocusGain(true)
                .setOnAudioFocusChangeListener { change ->
                    when (change) {
                        AudioManager.AUDIOFOCUS_LOSS,
                        AudioManager.AUDIOFOCUS_LOSS_TRANSIENT,
                        AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                            Log.d(TAG, "AudioFocus lost (change=$change) — pausing")
                            MediaSessionPlugin.handleMediaAction("pause")
                        }
                        AudioManager.AUDIOFOCUS_GAIN -> {
                            Log.d(TAG, "AudioFocus gained — resuming")
                            MediaSessionPlugin.handleMediaAction("play")
                        }
                        else -> Log.d(TAG, "AudioFocus change: $change")
                    }
                }
                .build()
            val result = am.requestAudioFocus(req)
            if (result == AudioManager.AUDIOFOCUS_REQUEST_GRANTED ||
                result == AudioManager.AUDIOFOCUS_REQUEST_DELAYED) {
                audioFocusRequest = req
                Log.d(TAG, "AudioFocus granted (result=$result)")
            } else {
                Log.w(TAG, "AudioFocus denied (result=$result)")
            }
        } else {
            @Suppress("DEPRECATION")
            am.requestAudioFocus(
                { change ->
                    when (change) {
                        AudioManager.AUDIOFOCUS_LOSS,
                        AudioManager.AUDIOFOCUS_LOSS_TRANSIENT,
                        AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                            Log.d(TAG, "AudioFocus lost (legacy, change=$change) — pausing")
                            MediaSessionPlugin.handleMediaAction("pause")
                        }
                        AudioManager.AUDIOFOCUS_GAIN -> {
                            Log.d(TAG, "AudioFocus gained (legacy) — resuming")
                            MediaSessionPlugin.handleMediaAction("play")
                        }
                    }
                },
                AudioManager.STREAM_MUSIC,
                AudioManager.AUDIOFOCUS_GAIN
            )
        }
    }

    private fun abandonAudioFocus() {
        val am = getSystemService(AUDIO_SERVICE) as AudioManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            audioFocusRequest?.let { am.abandonAudioFocusRequest(it) }
            audioFocusRequest = null
        } else {
            @Suppress("DEPRECATION")
            am.abandonAudioFocus(null)
        }
        Log.d(TAG, "AudioFocus abandoned")
    }

    // ── Becoming Noisy (headphone unplug / BT disconnect) ────────────────────

    private fun registerNoisyReceiver() {
        if (noisyReceiver != null) return
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent?) {
                if (intent?.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) {
                    Log.d(TAG, "Audio becoming noisy (headphones unplugged) — pausing")
                    MediaSessionPlugin.handleMediaAction("pause")
                }
            }
        }
        registerReceiver(receiver, IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY))
        noisyReceiver = receiver
        Log.d(TAG, "Noisy receiver registered")
    }

    private fun unregisterNoisyReceiver() {
        noisyReceiver?.let {
            try { unregisterReceiver(it) } catch (_: Exception) {}
            noisyReceiver = null
            Log.d(TAG, "Noisy receiver unregistered")
        }
    }
}
