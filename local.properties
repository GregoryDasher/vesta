package com.skipp.vesta

import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.widget.Button
import android.widget.RadioGroup
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import kotlin.concurrent.thread

class MainActivity : AppCompatActivity() {

    private lateinit var statusText: TextView
    private lateinit var actionButton: Button
    private lateinit var noDataText: TextView
    private lateinit var detailsText: TextView

    private lateinit var startModeGroup: RadioGroup
    private lateinit var fpsGroup: RadioGroup
    private lateinit var shadowGroup: RadioGroup
    private lateinit var touchControlsSwitch: Switch

    private var progressDialog: ProgressDialog? = null
    private var progressThread: Thread? = null
    @Volatile private var stopProgress = false

    private val prefs by lazy { getSharedPreferences("vesta", MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        statusText = findViewById(R.id.statusText)
        actionButton = findViewById(R.id.actionButton)
        noDataText = findViewById(R.id.noDataText)
        detailsText = findViewById(R.id.detailsText)

        startModeGroup = findViewById(R.id.startModeGroup)
        fpsGroup = findViewById(R.id.fpsGroup)
        shadowGroup = findViewById(R.id.shadowGroup)
        touchControlsSwitch = findViewById(R.id.touchControlsSwitch)

        actionButton.setOnClickListener { onActionButtonClicked() }

        startModeGroup.setOnCheckedChangeListener { _, id ->
            prefs.edit().putString("start_mode",
                if (id == R.id.modeStory) "story" else "sandbox").apply()
        }
        fpsGroup.setOnCheckedChangeListener { _, id ->
            val fps = when (id) {
                R.id.fps30 -> "30"; R.id.fps40 -> "40"
                R.id.fps50 -> "50"; R.id.fps60 -> "60"
                else -> "No limit"
            }
            prefs.edit().putString("fps", fps).apply()
        }
        shadowGroup.setOnCheckedChangeListener { _, id ->
            val q = when (id) {
                R.id.shadow0 -> 0; R.id.shadow1 -> 1; else -> 2
            }
            prefs.edit().putInt("shadow", q).apply()
        }
        touchControlsSwitch.setOnCheckedChangeListener { _, checked ->
            prefs.edit().putBoolean("touch", checked).apply()
        }

        restoreSettings()
    }

    override fun onResume() {
        super.onResume()
        refreshState()
    }

    private fun getMediaDirectory(): File? {
        val mediaDirs = getExternalMediaDirs()
        if (mediaDirs.isEmpty()) return null
        val mediaDir = mediaDirs[0]
        if (!mediaDir.exists() && !mediaDir.mkdirs()) return null
        return mediaDir
    }

    private fun refreshState() {
        val mediaDir = getMediaDirectory()
        if (mediaDir == null) {
            statusText.text = "Media dir unavailable"
            statusText.setTextColor(Color.parseColor("#C62828"))
            actionButton.visibility = Button.GONE
            noDataText.visibility = TextView.VISIBLE
            return
        }

        val result = GameFilesChecker.check(mediaDir)
        statusText.text = result.displayName

        if (result.allPresent) {
            statusText.setTextColor(Color.parseColor("#2E7D32"))
            actionButton.visibility = Button.VISIBLE
            actionButton.text = "Run"
            noDataText.visibility = TextView.GONE
        } else {
            statusText.setTextColor(Color.parseColor("#C62828"))
            actionButton.visibility = Button.GONE
            noDataText.visibility = TextView.VISIBLE
        }
        detailsText.text = result.details
    }

    private fun onActionButtonClicked() {
        val mediaDir = getMediaDirectory() ?: return
        val result = GameFilesChecker.check(mediaDir)

        if (!result.allPresent) {
            Toast.makeText(this, "Сначала скопируйте файлы игры", Toast.LENGTH_LONG).show()
            return
        }

        if (!WasmRuntime.isLoaded()) {
            Toast.makeText(this, "libmuguet.so не загружена", Toast.LENGTH_LONG).show()
            return
        }

        // Переключаем UI: скрываем панель, показываем SurfaceView
        val uiPanel = findViewById<android.view.View>(R.id.uiPanel)
        val gameSurface = findViewById<android.view.View>(R.id.gameSurface)
        uiPanel.visibility = android.view.View.GONE
        gameSurface.visibility = android.view.View.VISIBLE

        // Сбрасываем прошлый прогресс
        File(mediaDir, "progress.txt").writeText("PROGRESS|0|Старт...")

        // Прогресс-диалог как и был
        progressDialog = ProgressDialog(this)
        progressDialog?.show {
            stopProgress = true
            progressDialog?.dismiss()
            uiPanel.visibility = android.view.View.VISIBLE
            gameSurface.visibility = android.view.View.GONE
        }

        stopProgress = false
        progressThread = thread {
            while (!stopProgress) {
                try {
                    val f = File(mediaDir, "progress.txt")
                    if (f.exists()) {
                        val line = f.readText().trim()
                        val parts = line.split("|", limit = 3)
                        if (parts.size == 3) {
                            val status = parts[0]
                            val percent = parts[1].toIntOrNull() ?: 0
                            val message = parts[2]
                            runOnUiThread {
                                when (status) {
                                    "PROGRESS" -> progressDialog?.update(percent, message)
                                    "OK" -> {
                                        progressDialog?.update(100, message)
                                        android.os.Handler(mainLooper).postDelayed({
                                            progressDialog?.dismiss()
                                            progressDialog = null
                                        }, 1500)
                                        stopProgress = true
                                    }
                                    "ERR" -> {
                                        progressDialog?.setError(message)
                                        stopProgress = true
                                    }
                                }
                            }
                        }
                    }
                    Thread.sleep(200)
                } catch (_: InterruptedException) {
                    break
                } catch (e: Throwable) {
                    Log.e("Vesta", "Ошибка чтения прогресса: ${e.message}")
                }
            }
        }

        // Запускаем движок
        thread {
            try {
                val code = WasmRuntime.nativeRun(mediaDir.absolutePath)
                Log.i("Vesta", "nativeRun вернул: $code")
            } catch (e: Throwable) {
                Log.e("Vesta", "nativeRun упал: ${e.message}", e)
                runOnUiThread {
                    progressDialog?.setError(e.message ?: "Unknown error")
                }
            }
        }
    }

    private fun restoreSettings() {
        when (prefs.getString("start_mode", "story")) {
            "sandbox" -> startModeGroup.check(R.id.modeSandbox)
            else -> startModeGroup.check(R.id.modeStory)
        }
        when (prefs.getString("fps", "60")) {
            "30" -> fpsGroup.check(R.id.fps30)
            "40" -> fpsGroup.check(R.id.fps40)
            "50" -> fpsGroup.check(R.id.fps50)
            "60" -> fpsGroup.check(R.id.fps60)
            else -> fpsGroup.check(R.id.fpsNoLimit)
        }
        when (prefs.getInt("shadow", 1)) {
            0 -> shadowGroup.check(R.id.shadow0)
            1 -> shadowGroup.check(R.id.shadow1)
            else -> shadowGroup.check(R.id.shadow2)
        }
        touchControlsSwitch.isChecked = prefs.getBoolean("touch", false)
    }
}