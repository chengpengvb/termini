package com.termini.www

import android.app.Activity
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.hardware.input.InputManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.InputDevice
import android.view.View
import android.view.WindowManager
import android.view.inputmethod.InputMethodManager
import android.webkit.JavascriptInterface
import android.webkit.MimeTypeMap
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.math.roundToInt

class MainActivity : TauriActivity() {
  private var webViewRef: WebView? = null
  private var lastTopPx: Int = 0
  private var lastRightPx: Int = 0
  private var lastBottomPx: Int = 0
  private var lastLeftPx: Int = 0
  private var lastImePx: Int = 0
  private var lastHardKeyboard: Boolean = false
  private var lastHardMouse: Boolean = false
  private val mainHandler = Handler(Looper.getMainLooper())
  private val inputDeviceListener = object : InputManager.InputDeviceListener {
    override fun onInputDeviceAdded(deviceId: Int) {
      refreshHardDevices()
    }

    override fun onInputDeviceRemoved(deviceId: Int) {
      refreshHardDevices()
    }

    override fun onInputDeviceChanged(deviceId: Int) {
      refreshHardDevices()
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    WebView.setWebContentsDebuggingEnabled(true)
    window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
    val rootView: View = findViewById(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(rootView) { _, insets ->
      val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
      val displayCutout = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
      val density = resources.displayMetrics.density
      val safeTop = maxOf(systemBars.top, displayCutout.top)
      val safeBottom = maxOf(systemBars.bottom, displayCutout.bottom)
      val safeLeft = maxOf(systemBars.left, displayCutout.left)
      val safeRight = maxOf(systemBars.right, displayCutout.right)
      lastTopPx = (safeTop / density).roundToInt()
      lastBottomPx = (safeBottom / density).roundToInt()
      lastLeftPx = (safeLeft / density).roundToInt()
      lastRightPx = (safeRight / density).roundToInt()
      lastImePx = (ime.bottom / density).roundToInt()
      scheduleInjection()
      insets
    }
    val inputManager = getSystemService(Context.INPUT_SERVICE) as InputManager
    inputManager.registerInputDeviceListener(inputDeviceListener, mainHandler)
    refreshHardDevices()
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        val webView = webViewRef
        if (webView == null) {
          finishAffinity()
          return
        }
        webView.evaluateJavascript("(function(){try{return window.__tmConsumeBack&&window.__tmConsumeBack()?true:false;}catch(e){return false;}})()") { result ->
          if (result == "true") {
            return@evaluateJavascript
          }
          finishAffinity()
        }
      }
    })
  }

  override fun onDestroy() {
    val inputManager = getSystemService(Context.INPUT_SERVICE) as InputManager
    inputManager.unregisterInputDeviceListener(inputDeviceListener)
    super.onDestroy()
  }

  override fun onResume() {
    super.onResume()
    refreshHardDevices(true)
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    refreshHardDevices()
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webViewRef = webView
    webView.setBackgroundColor(0x00000000)
    webView.addJavascriptInterface(TerminiNativeBridge(), "TerminiNative")
    webView.requestApplyInsets()
    scheduleInjection()
  }

  private fun refreshHardDevices(force: Boolean = false) {
    val keyboard = resolveHardKeyboardConnected()
    val mouse = resolveHardMouseConnected()
    if (!force && keyboard == lastHardKeyboard && mouse == lastHardMouse) {
      return
    }
    lastHardKeyboard = keyboard
    lastHardMouse = mouse
    Log.i("TerminiInput", "hard devices keyboard=$keyboard mouse=$mouse")
    scheduleInjection()
  }

  private fun resolveHardKeyboardConnected(): Boolean {
    for (id in InputDevice.getDeviceIds()) {
      val device = InputDevice.getDevice(id) ?: continue
      if (isExternalAlphabeticKeyboard(device)) {
        return true
      }
    }
    return false
  }

  private fun resolveHardMouseConnected(): Boolean {
    for (id in InputDevice.getDeviceIds()) {
      val device = InputDevice.getDevice(id) ?: continue
      if (isExternalMouse(device)) {
        return true
      }
    }
    return false
  }

  private fun isExternalMouse(device: InputDevice): Boolean {
    val mouse = device.supportsSource(InputDevice.SOURCE_MOUSE) || device.supportsSource(InputDevice.SOURCE_MOUSE_RELATIVE)
    if (!mouse) {
      return false
    }
    // Xiaomi Pad reports its real detachable mouse/touchpad as isExternal=false.
    // isVirtual is the reliable distinction from Android's synthetic mouse device.
    return !device.isVirtual
  }

  private fun isExternalAlphabeticKeyboard(device: InputDevice): Boolean {
    if (!device.supportsSource(InputDevice.SOURCE_KEYBOARD)) {
      return false
    }
    if (device.keyboardType != InputDevice.KEYBOARD_TYPE_ALPHABETIC) {
      return false
    }
    // Detachable pogo keyboards are also reported as isExternal=false on Xiaomi Pad.
    return !device.isVirtual
  }

  private fun scheduleInjection() {
    val delays = longArrayOf(0L, 150L, 500L, 1500L, 3000L)
    for (delay in delays) {
      mainHandler.postDelayed({ injectSafeAreaIntoWebView() }, delay)
    }
  }

  private fun injectSafeAreaIntoWebView() {
    val webView = webViewRef ?: return
    val hard = if (lastHardKeyboard) 1 else 0
    val mouse = if (lastHardMouse) 1 else 0
    val script = """
      (function () {
        if (!document.documentElement) {
          return;
        }
        var s = document.documentElement.style;
        s.setProperty('--tm-safe-area-inset-top', '${lastTopPx}px');
        s.setProperty('--tm-safe-area-inset-right', '${lastRightPx}px');
        s.setProperty('--tm-safe-area-inset-bottom', '${lastBottomPx}px');
        s.setProperty('--tm-safe-area-inset-left', '${lastLeftPx}px');
        s.setProperty('--tm-ime-inset', '${lastImePx}px');
        s.setProperty('--tm-hard-keyboard', '${hard}');
        s.setProperty('--tm-hard-mouse', '${mouse}');
        document.documentElement.classList.toggle('tm-hard-keyboard', ${hard} === 1);
        document.documentElement.classList.toggle('tm-hard-mouse', ${mouse} === 1);
      })();
    """.trimIndent()
    webView.evaluateJavascript(script, null)
  }

  private inner class TerminiNativeBridge {
    @JavascriptInterface
    fun exitApp() {
      mainHandler.post { finishAffinity() }
    }

    @JavascriptInterface
    fun showSoftInput() {
      mainHandler.post {
        val webView = webViewRef ?: return@post
        webView.requestFocus()
        val imm = getSystemService(InputMethodManager::class.java)
        imm.showSoftInput(webView, 0)
      }
    }

    @JavascriptInterface
    fun hideSoftInput() {
      mainHandler.post {
        val webView = webViewRef ?: return@post
        val imm = getSystemService(InputMethodManager::class.java)
        imm.hideSoftInputFromWindow(webView.windowToken, 0)
      }
    }
  }

  companion object {
    @JvmStatic
    fun openLocalFile(activity: Activity, path: String, openWith: Boolean): String {
      val latch = CountDownLatch(1)
      var result = "ERROR:open not started"
      activity.runOnUiThread {
        result = try {
          startLocalFileIntent(activity, path, openWith)
          "OK"
        } catch (error: Exception) {
          "ERROR:${error.message}"
        }
        latch.countDown()
      }
      if (!latch.await(15, TimeUnit.SECONDS)) {
        return "ERROR:open timeout"
      }
      return result
    }

    private fun startLocalFileIntent(activity: Activity, path: String, openWith: Boolean) {
      val file = File(path)
      if (!file.exists()) {
        throw IllegalStateException("file missing: $path")
      }
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", file)
      val ext = file.extension.lowercase()
      if (ext.isEmpty()) {
        throw IllegalStateException("file extension missing")
      }
      val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext)
      if (mime == null) {
        throw IllegalStateException("mime type missing: $ext")
      }
      val view = Intent(Intent.ACTION_VIEW)
      view.setDataAndType(uri, mime)
      view.clipData = ClipData.newRawUri("", uri)
      view.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
      if (openWith) {
        val chooser = Intent.createChooser(view, null)
        chooser.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        activity.startActivity(chooser)
        return
      }
      activity.startActivity(view)
    }
  }
}
