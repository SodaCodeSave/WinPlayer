package app.tauri.mediasession

import android.Manifest
import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import android.os.Build
import android.os.SystemClock
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import android.util.Log
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.media.app.NotificationCompat as MediaNotificationCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.net.HttpURLConnection
import java.net.URL

@InvokeArg
class UpdateStateArgs {
    var title: String? = null
    var artist: String? = null
    var album: String? = null
    var artworkUrl: String? = null
    var duration: Double? = null
    var position: Double? = null
    var playbackSpeed: Double? = null
    var isPlaying: Boolean? = null
    var canPrev: Boolean? = null
    var canNext: Boolean? = null
    var canSeek: Boolean? = null
}

@InvokeArg
class UpdateTimelineArgs {
    var position: Double? = null
    var duration: Double? = null
    var playbackSpeed: Double? = null
}

@TauriPlugin
class MediaSessionPlugin(private val activity: Activity) : Plugin(activity) {

    init {
        activeInstance = this
        Log.d(TAG, "init")
        cancelNotificationArtifacts(activity.applicationContext)
    }

    // Native session & notification
    private var mediaSession: MediaSessionCompat? = null
    private var notificationManager: NotificationManagerCompat? = null

    // Artwork cache
    private var cachedArtworkUrl: String? = null
    private var cachedArtworkBitmap: Bitmap? = null
    private var fallbackArtworkBitmap: Bitmap? = null
    // Tracks the URL currently being downloaded — intentionally NOT cleared by artworkUrl=""
    // so that in-flight downloads survive the JS clear→url race condition.
    private var downloadingArtworkUrl: String? = null

    // Permission bookkeeping
    private var notificationPermissionRequested = false

    // Merge state — omitted fields in updateState keep their previous values
    private var currentTitle: String = ""
    private var currentArtist: String = ""
    private var currentAlbum: String = ""
    private var currentDuration: Double = 0.0
    private var currentPosition: Double = 0.0
    private var currentPlaybackSpeed: Double = 1.0
    private var currentIsPlaying: Boolean = false
    private var currentCanPrev: Boolean = false
    private var currentCanNext: Boolean = false
    private var currentCanSeek: Boolean = true

    // Dynamic identifiers derived from the host app
    private val channelId: String by lazy { "${activity.packageName}.media" }
    private val sessionTag: String by lazy { "${activity.packageName}.MediaSession" }

    // ── Commands ────────────────────────────────────────────────────────

    @Command
    fun initialize(invoke: Invoke) {
        Log.d(TAG, "initialize: requesting permissions and creating session")
        requestNotificationPermission()
        ensureSession()
        invoke.resolve()
    }

    @Command
    fun updateState(invoke: Invoke) {
        val args = invoke.parseArgs(UpdateStateArgs::class.java)

        // Log received fields (only the ones actually provided)
        val fields = mutableListOf<String>()
        args.title?.let { fields.add("title=\"$it\"") }
        args.artist?.let { fields.add("artist=\"$it\"") }
        args.album?.let { fields.add("album=\"$it\"") }
        args.artworkUrl?.let { fields.add("artworkUrl=\"$it\"") }
        args.duration?.let { fields.add("duration=${it}s") }
        args.position?.let { fields.add("position=${it}s") }
        args.playbackSpeed?.let { fields.add("playbackSpeed=${it}x") }
        args.isPlaying?.let { fields.add("isPlaying=$it") }
        args.canPrev?.let { fields.add("canPrev=$it") }
        args.canNext?.let { fields.add("canNext=$it") }
        args.canSeek?.let { fields.add("canSeek=$it") }
        Log.d(TAG, "updateState: received { ${fields.joinToString(", ")} }")

        val session = ensureSession() ?: run {
            Log.e(TAG, "updateState: media session unavailable")
            invoke.reject("media session unavailable")
            return
        }

        requestNotificationPermission()

        // ── Merge incoming args with stored state ───────────────────
        args.title?.trim()?.let { currentTitle = it }
        args.artist?.trim()?.let { currentArtist = it }
        args.album?.trim()?.let { currentAlbum = it }
        args.duration?.let { currentDuration = it }
        args.position?.let { currentPosition = it }
        args.playbackSpeed?.let { currentPlaybackSpeed = it }
        args.isPlaying?.let { currentIsPlaying = it }
        args.canPrev?.let { currentCanPrev = it }
        args.canNext?.let { currentCanNext = it }
        args.canSeek?.let { currentCanSeek = it }

        val artworkUrl = args.artworkUrl

        if (artworkUrl != null) {
            if (artworkUrl.isEmpty()) {
                Log.d(TAG, "updateState: clearing artwork (URL)")
                applyFallbackArtwork(null)
                cachedArtworkUrl = null
                downloadingArtworkUrl = null
            } else if (artworkUrl != cachedArtworkUrl) {
                Log.d(TAG, "updateState: downloading artwork from URL")
                cachedArtworkUrl = artworkUrl
                downloadingArtworkUrl = artworkUrl
                downloadAndApplyArtwork(artworkUrl)
            }
        }

        // ── Metadata + playback state + notification ─────────────────
        val metadata = buildMetadata()
        session.setMetadata(metadata)
        session.setPlaybackState(buildPlaybackState())
        session.isActive = true
        val notification = updateNotification(metadata)

        if (notification != null) MediaSessionCleanupService.start(activity, notification)

        Log.d(TAG, "updateState: applied -> \"$currentTitle\" by $currentArtist, " +
            "playing=$currentIsPlaying, pos=${currentPosition}s/${currentDuration}s, " +
            "artwork=${if (cachedArtworkBitmap != null) "yes" else "none"}")
        invoke.resolve()
    }

    @Command
    fun updateTimeline(invoke: Invoke) {
        val args = invoke.parseArgs(UpdateTimelineArgs::class.java)

        val session = mediaSession ?: run {
            Log.e(TAG, "updateTimeline: session not initialized — call updateState first")
            invoke.reject("media session not initialized — call updateState first")
            return
        }

        args.position?.let { currentPosition = it }
        args.duration?.let { currentDuration = it }
        args.playbackSpeed?.let { currentPlaybackSpeed = it }

        session.setPlaybackState(buildPlaybackState())
        if (args.duration != null) session.setMetadata(buildMetadata())

        invoke.resolve()
    }

    @Command
    fun clear(invoke: Invoke) {
        Log.d(TAG, "clear: releasing session and dismissing notification")
        releaseSession()
        invoke.resolve()
    }

    // ── Lifecycle ───────────────────────────────────────────────────────

    override fun onDestroy() {
        Log.d(TAG, "onDestroy: cleaning up")
        releaseSession()
        activeInstance = null
        super.onDestroy()
    }

    // ── State builders ──────────────────────────────────────────────────

    private fun buildPlaybackState(): PlaybackStateCompat {
        val positionMs = (currentPosition * 1000.0).toLong()
        val state = if (currentIsPlaying) PlaybackStateCompat.STATE_PLAYING
                    else PlaybackStateCompat.STATE_PAUSED
        val speed = if (currentIsPlaying) currentPlaybackSpeed.toFloat() else 0.0f

        return PlaybackStateCompat.Builder()
            .setActions(buildAvailableActions())
            .setState(state, positionMs, speed, SystemClock.elapsedRealtime())
            .build()
    }

    private fun buildAvailableActions(): Long {
        var actions = PlaybackStateCompat.ACTION_PLAY_PAUSE or
            PlaybackStateCompat.ACTION_PLAY or
            PlaybackStateCompat.ACTION_PAUSE or
            PlaybackStateCompat.ACTION_STOP
        if (currentCanSeek) actions = actions or PlaybackStateCompat.ACTION_SEEK_TO
        if (currentCanPrev) actions = actions or PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS
        if (currentCanNext) actions = actions or PlaybackStateCompat.ACTION_SKIP_TO_NEXT
        return actions
    }

    private fun buildMetadata(): MediaMetadataCompat {
        val builder = MediaMetadataCompat.Builder()
        if (currentTitle.isNotEmpty()) {
            builder.putString(MediaMetadataCompat.METADATA_KEY_TITLE, currentTitle)
        }
        if (currentArtist.isNotEmpty()) {
            builder.putString(MediaMetadataCompat.METADATA_KEY_ARTIST, currentArtist)
        }
        if (currentAlbum.isNotEmpty()) {
            builder.putString(MediaMetadataCompat.METADATA_KEY_ALBUM, currentAlbum)
        }
        val durationMs = (currentDuration * 1000.0).toLong()
        if (durationMs > 0) {
            builder.putLong(MediaMetadataCompat.METADATA_KEY_DURATION, durationMs)
        }
        cachedArtworkBitmap?.let {
            builder.putBitmap(MediaMetadataCompat.METADATA_KEY_ALBUM_ART, it)
        }
        return builder.build()
    }

    // ── Internal helpers ────────────────────────────────────────────────

    private fun releaseSession() {
        try {
            emitAction("pause")
        } catch (_: Throwable) {
        }
        mediaSession?.let { session ->
            try {
                val stoppedState = PlaybackStateCompat.Builder()
                    .setActions(0)
                    .setState(PlaybackStateCompat.STATE_NONE, 0L, 0f)
                    .build()
                session.setPlaybackState(stoppedState)
                session.setMetadata(MediaMetadataCompat.Builder().build())
            } catch (_: Throwable) {
            }
            session.isActive = false
            session.release()
        }
        mediaSession = null
        notificationManager?.cancel(MediaSessionCleanupService.NOTIFICATION_ID)
        cancelNotificationArtifacts(activity.applicationContext)
        MediaSessionCleanupService.stop()

        currentTitle = ""; currentArtist = ""; currentAlbum = ""
        currentDuration = 0.0; currentPosition = 0.0; currentPlaybackSpeed = 1.0
        currentIsPlaying = false; currentCanPrev = false; currentCanNext = false; currentCanSeek = true

        recycleCachedArtworkBitmap()
        cachedArtworkUrl = null
        downloadingArtworkUrl = null
    }

    private fun requestNotificationPermission() {
        if (notificationPermissionRequested) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED
            ) {
                ActivityCompat.requestPermissions(
                    activity,
                    arrayOf(Manifest.permission.POST_NOTIFICATIONS),
                    NOTIFICATION_PERMISSION_REQUEST_CODE
                )
            }
        }
        notificationPermissionRequested = true
    }

    @Suppress("DEPRECATION")
    private fun ensureSession(): MediaSessionCompat? {
        if (mediaSession != null) return mediaSession

        Log.d(TAG, "ensureSession: creating new session (tag=$sessionTag, channel=$channelId)")
        val session = MediaSessionCompat(activity, sessionTag)
        session.setCallback(sessionCallback)
        session.setFlags(
            MediaSessionCompat.FLAG_HANDLES_MEDIA_BUTTONS or
                MediaSessionCompat.FLAG_HANDLES_TRANSPORT_CONTROLS
        )
        session.isActive = true

        val launchIntent = activity.packageManager.getLaunchIntentForPackage(activity.packageName)
        if (launchIntent != null) {
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0
            session.setSessionActivity(PendingIntent.getActivity(activity, 0, launchIntent, flags))
        }

        mediaSession = session
        notificationManager = NotificationManagerCompat.from(activity)
        createNotificationChannel()
        return mediaSession
    }

    // ── Artwork download (native HTTP, no CORS) ─────────────────────────

    private fun downloadAndApplyArtwork(url: String) {
        Thread {
            try {
                val bitmap = fetchBitmapFromImageUrl(url)
                if (bitmap != null) {
                    Log.d(TAG, "downloadArtwork: decoded ${bitmap.width}x${bitmap.height} from $url")
                    activity.runOnUiThread {
                        // Apply if this is still the most recently requested URL.
                        // Using downloadingArtworkUrl (not cachedArtworkUrl) so the result
                        // survives an artworkUrl="" clear that arrived after the download started.
                        if (downloadingArtworkUrl == url) {
                            downloadingArtworkUrl = null
                            recycleCachedArtworkBitmap()
                            cachedArtworkBitmap = bitmap
                            cachedArtworkUrl = url
                            val session = mediaSession ?: return@runOnUiThread
                            val metadata = buildMetadata()
                            session.setMetadata(metadata)
                            val newNotif = updateNotification(metadata)
                            if (newNotif != null) MediaSessionCleanupService.pendingNotification = newNotif
                        } else {
                            downloadingArtworkUrl = null
                            bitmap.recycle()
                        }
                    }
                } else {
                    Log.w(TAG, "downloadArtwork: decode returned null for $url")
                    applyFallbackArtwork(url)
                }
            } catch (e: Exception) {
                Log.w(TAG, "downloadArtwork: failed for $url: ${e.message}")
                applyFallbackArtwork(url)
            }
        }.start()
    }

    private fun fetchBitmapFromImageUrl(url: String): Bitmap? {
        var connection: HttpURLConnection? = null
        return try {
            connection = URL(url).openConnection() as HttpURLConnection
            connection.connectTimeout = 10000
            connection.readTimeout = 10000
            connection.instanceFollowRedirects = true
            connection.connect()
            val contentType = connection.contentType?.lowercase() ?: ""
            if (contentType.startsWith("video/")) {
                null
            } else {
                val bytes = connection.inputStream.use { it.readBytes() }
                decodeSampledBitmap(bytes, MAX_ARTWORK_SIZE)
            }
        } catch (_: Exception) {
            null
        } finally {
            connection?.disconnect()
        }
    }

    private fun applyFallbackArtwork(expectedUrl: String?) {
        activity.runOnUiThread {
            if (expectedUrl != null && downloadingArtworkUrl != expectedUrl) return@runOnUiThread
            downloadingArtworkUrl = null
            cachedArtworkUrl = null
            recycleCachedArtworkBitmap()
            cachedArtworkBitmap = getFallbackArtworkBitmap()

            val session = mediaSession ?: return@runOnUiThread
            val metadata = buildMetadata()
            session.setMetadata(metadata)
            val newNotif = updateNotification(metadata)
            if (newNotif != null) MediaSessionCleanupService.pendingNotification = newNotif
        }
    }

    private fun recycleCachedArtworkBitmap() {
        val bitmap = cachedArtworkBitmap
        if (bitmap != null && bitmap !== fallbackArtworkBitmap) {
            bitmap.recycle()
        }
        cachedArtworkBitmap = null
    }

    private fun getFallbackArtworkBitmap(): Bitmap? {
        if (fallbackArtworkBitmap != null) return fallbackArtworkBitmap
        return try {
            val drawable = activity.packageManager.getApplicationIcon(activity.applicationInfo)
            val bitmap = drawableToBitmap(drawable)
            val scaled = scaleBitmapToMax(bitmap, MAX_ARTWORK_SIZE)
            fallbackArtworkBitmap = scaled
            fallbackArtworkBitmap
        } catch (_: Throwable) {
            null
        }
    }

    private fun drawableToBitmap(drawable: Drawable): Bitmap {
        if (drawable is BitmapDrawable && drawable.bitmap != null) {
            return drawable.bitmap
        }
        val width = if (drawable.intrinsicWidth > 0) drawable.intrinsicWidth else MAX_ARTWORK_SIZE
        val height = if (drawable.intrinsicHeight > 0) drawable.intrinsicHeight else MAX_ARTWORK_SIZE
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        drawable.setBounds(0, 0, canvas.width, canvas.height)
        drawable.draw(canvas)
        return bitmap
    }

    private fun scaleBitmapToMax(source: Bitmap, maxSize: Int): Bitmap {
        val width = source.width
        val height = source.height
        if (width <= maxSize && height <= maxSize) return source

        val widthRatio = maxSize.toFloat() / width.toFloat()
        val heightRatio = maxSize.toFloat() / height.toFloat()
        val scale = if (widthRatio < heightRatio) widthRatio else heightRatio
        val outWidth = (width * scale).toInt().coerceAtLeast(1)
        val outHeight = (height * scale).toInt().coerceAtLeast(1)
        val scaled = Bitmap.createScaledBitmap(source, outWidth, outHeight, true)
        if (scaled !== source) source.recycle()
        return scaled
    }

    // ── Notification action PendingIntents ───────────────────────────────

    private fun buildActionPendingIntent(action: String, requestCode: Int): PendingIntent {
        val intent = Intent(activity, MediaActionReceiver::class.java)
            .putExtra(MediaActionReceiver.EXTRA_ACTION, action)
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0
        return PendingIntent.getBroadcast(activity, requestCode, intent, flags)
    }

    private fun getSmallIcon(): Int {
        val res = activity.resources
        val pkg = activity.packageName
        var id = res.getIdentifier("ic_notification", "drawable", pkg)
        if (id != 0) return id
        id = res.getIdentifier("ic_notification", "mipmap", pkg)
        if (id != 0) return id
        id = res.getIdentifier("ic_launcher_foreground", "drawable", pkg)
        if (id != 0) return id
        return android.R.drawable.ic_media_play
    }

    // ── Notification ────────────────────────────────────────────────────

    private fun updateNotification(metadata: MediaMetadataCompat): Notification? {
        val session = mediaSession ?: return null
        val manager = notificationManager ?: return null

        val title = metadata.getString(MediaMetadataCompat.METADATA_KEY_TITLE)
            ?: activity.applicationInfo.loadLabel(activity.packageManager).toString()
        val artist = metadata.getString(MediaMetadataCompat.METADATA_KEY_ARTIST)
        val album = metadata.getString(MediaMetadataCompat.METADATA_KEY_ALBUM)
        val subtitle = listOfNotNull(artist, album).filter { it.isNotBlank() }.joinToString(" \u2014 ")
        val artwork = metadata.getBitmap(MediaMetadataCompat.METADATA_KEY_ALBUM_ART)

        val actions = mutableListOf<NotificationCompat.Action>()
        if (currentCanPrev) {
            actions.add(NotificationCompat.Action(
                android.R.drawable.ic_media_previous, "Previous",
                buildActionPendingIntent("previous", RC_PREV)
            ))
        }
        actions.add(
            if (currentIsPlaying) {
                NotificationCompat.Action(
                    android.R.drawable.ic_media_pause, "Pause",
                    buildActionPendingIntent("pause", RC_PAUSE)
                )
            } else {
                NotificationCompat.Action(
                    android.R.drawable.ic_media_play, "Play",
                    buildActionPendingIntent("play", RC_PLAY)
                )
            }
        )
        if (currentCanNext) {
            actions.add(NotificationCompat.Action(
                android.R.drawable.ic_media_next, "Next",
                buildActionPendingIntent("next", RC_NEXT)
            ))
        }

        val builder = NotificationCompat.Builder(activity, channelId)
            .setSmallIcon(getSmallIcon())
            .setContentTitle(title)
            .setContentText(subtitle)
            .setCategory(NotificationCompat.CATEGORY_TRANSPORT)
            .setOnlyAlertOnce(true)
            .setShowWhen(false)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setOngoing(true)
            .setSilent(true)
            .setColorized(true)

        if (artwork != null) builder.setLargeIcon(artwork)

        val launchIntent = activity.packageManager.getLaunchIntentForPackage(activity.packageName)
        if (launchIntent != null) {
            val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0
            builder.setContentIntent(PendingIntent.getActivity(activity, 0, launchIntent, flags))
        }

        val compactIndices = IntArray(actions.size.coerceAtMost(3)) { it }
        val style = MediaNotificationCompat.MediaStyle()
            .setMediaSession(session.sessionToken)
        if (compactIndices.isNotEmpty()) {
            style.setShowActionsInCompactView(*compactIndices)
        }
        builder.setStyle(style)
        actions.forEach { builder.addAction(it) }

        val notification = builder.build()
        manager.notify(MediaSessionCleanupService.NOTIFICATION_ID, notification)
        return notification
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = activity.getSystemService(NotificationManager::class.java) ?: return
        if (manager.getNotificationChannel(channelId) != null) return
        val channel = NotificationChannel(
            channelId, "Media playback", NotificationManager.IMPORTANCE_LOW
        ).apply { description = "Media playback controls" }
        manager.createNotificationChannel(channel)
    }

    // ── Events → JavaScript ─────────────────────────────────────────────

    internal fun emitAction(action: String) {
        Log.d(TAG, "emit: action=\"$action\"")
        val payload = JSObject()
        payload.put("action", action)
        activity.runOnUiThread { trigger("media_action", payload) }
    }

    private fun emitSeek(positionMs: Long) {
        val seekSeconds = positionMs / 1000.0
        Log.d(TAG, "emit: action=\"seek\", seekPosition=${seekSeconds}s")
        val payload = JSObject()
        payload.put("action", "seek")
        payload.put("seekPosition", seekSeconds)
        activity.runOnUiThread { trigger("media_action", payload) }
    }

    private val sessionCallback = object : MediaSessionCompat.Callback() {
        override fun onPlay() = emitAction("play")
        override fun onPause() = emitAction("pause")
        override fun onStop() = emitAction("stop")
        override fun onSkipToNext() = emitAction("next")
        override fun onSkipToPrevious() = emitAction("previous")
        override fun onSeekTo(pos: Long) = emitSeek(pos)
    }

    // ── Bitmap helpers ──────────────────────────────────────────────────

    private fun decodeSampledBitmap(bytes: ByteArray, maxSize: Int): Bitmap? {
        val opts = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, opts)
        opts.inSampleSize = calculateInSampleSize(opts.outWidth, opts.outHeight, maxSize)
        opts.inJustDecodeBounds = false
        return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, opts)
    }

    private fun calculateInSampleSize(width: Int, height: Int, maxSize: Int): Int {
        var sample = 1
        var w = width; var h = height
        while (w / 2 >= maxSize && h / 2 >= maxSize) { sample *= 2; w /= 2; h /= 2 }
        return sample
    }

    companion object {
        private const val TAG = "plugin/media-session"
        private const val NOTIFICATION_PERMISSION_REQUEST_CODE = 9402
        private const val MAX_ARTWORK_SIZE = 512
        private const val RC_PLAY = 1
        private const val RC_PAUSE = 2
        private const val RC_NEXT = 3
        private const val RC_PREV = 4

        @Volatile
        internal var activeInstance: MediaSessionPlugin? = null

        private fun cancelNotificationArtifacts(context: Context, hard: Boolean = false) {
            try {
                NotificationManagerCompat.from(context).cancel(MediaSessionCleanupService.NOTIFICATION_ID)
            } catch (_: Throwable) {
            }

            try {
                val manager = context.getSystemService(NotificationManager::class.java) ?: return
                manager.cancel(MediaSessionCleanupService.NOTIFICATION_ID)

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    val expectedChannel = "${context.packageName}.media"
                    for (status in manager.activeNotifications) {
                        val notification = status.notification
                        val sameId = status.id == MediaSessionCleanupService.NOTIFICATION_ID
                        val sameCategory = notification.category == NotificationCompat.CATEGORY_TRANSPORT
                        val sameChannel = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                            notification.channelId == expectedChannel
                        } else {
                            false
                        }
                        if (sameId || sameCategory || sameChannel) {
                            manager.cancel(status.tag, status.id)
                        }
                    }
                }

                if (hard && Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    val expectedChannel = "${context.packageName}.media"
                    if (manager.getNotificationChannel(expectedChannel) != null) {
                        manager.deleteNotificationChannel(expectedChannel)
                    }
                }
            } catch (_: Throwable) {
            }
        }

        internal fun forceCleanup(context: Context) {
            val plugin = activeInstance
            if (plugin != null) {
                try {
                    plugin.emitAction("pause")
                } catch (_: Throwable) {
                }
                plugin.releaseSession()
            }
            cancelNotificationArtifacts(context, hard = true)
        }

        internal fun handleMediaAction(action: String) {
            val plugin = activeInstance
            if (plugin == null) {
                Log.w(TAG, "handleMediaAction: no active plugin instance, ignoring \"$action\"")
                return
            }
            Log.d(TAG, "handleMediaAction: dispatching \"$action\" → JS")
            plugin.emitAction(action)
        }
    }
}
