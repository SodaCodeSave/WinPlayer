package app.tauri.mediasession

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * Manifest-declared receiver for notification media button taps.
 *
 * Using an explicit-intent receiver (declared in the manifest) is the most
 * reliable way to handle notification action buttons across all API levels.
 * Runtime-registered receivers can miss broadcasts from PendingIntents fired
 * by the system notification framework.
 */
class MediaActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent?) {
        val action = intent?.getStringExtra(EXTRA_ACTION) ?: return
        Log.d(TAG, "onReceive: action=\"$action\"")
        MediaSessionPlugin.handleMediaAction(action)
    }

    companion object {
        internal const val EXTRA_ACTION = "app.tauri.mediasession.EXTRA_ACTION"
        private const val TAG = "plugin/media-session"
    }
}
