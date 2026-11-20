package io.github.teamwicked.vessel

import android.app.AlertDialog
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Only a build made with VESSEL_WEB_INSPECTOR=1 lets Chrome's inspector
    // attach to the page over USB debugging, to diagnose it on a phone.
    // Builds given out are made without it: the page holds the way to
    // every paired machine.
    if (BuildConfig.WEB_INSPECTOR) {
      WebView.setWebContentsDebuggingEnabled(true)
    }
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    keepClearOfBarsAndKeyboard()
    warnIfWebViewIsOld()
  }

  // Edge to edge (which Android 15 imposes on apps targeting it) draws the
  // web view under the status bar, the navigation bar and the keyboard,
  // and the page knows none of their sizes: its header sat under the
  // status bar and the keyboard covered the composer and the dialogs'
  // buttons (seen in the emulator). The content is padded by the system
  // bars, the display cutout and the keyboard instead, so the page always
  // has the visible rectangle, as in a browser.
  private fun keepClearOfBarsAndKeyboard() {
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val clear = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or
          WindowInsetsCompat.Type.ime()
      )
      view.setPadding(clear.left, clear.top, clear.right, clear.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }

  // The page is tried on Android System WebView 113 (the oldest seen
  // working); an older one may draw a blank page. A phone without Play
  // updates can be that far behind, so say so instead of showing white,
  // and offer the store page where the WebView is updated.
  private fun warnIfWebViewIsOld() {
    val version = WebView.getCurrentWebViewPackage()?.versionName ?: return
    val major = version.substringBefore('.').toIntOrNull() ?: return
    if (major >= MIN_WEBVIEW) return
    AlertDialog.Builder(this)
      .setTitle("Update Android System WebView")
      .setMessage(
        "Vessel needs Android System WebView $MIN_WEBVIEW or newer; this phone has $version. " +
          "Update it from the Play Store, then open Vessel again."
      )
      .setPositiveButton("Open the store") { _, _ ->
        val store = Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=com.google.android.webview"))
        try { startActivity(store) } catch (_: ActivityNotFoundException) {}
      }
      .setNegativeButton("Continue anyway", null)
      .show()
  }

  private companion object {
    const val MIN_WEBVIEW = 113
  }
}
