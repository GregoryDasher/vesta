package com.skipp.vesta

import android.app.AlertDialog
import android.content.Context
import android.view.LayoutInflater
import android.view.View
import android.widget.ProgressBar
import android.widget.TextView

/**
 * Маленькое окно с прогресс-баром.
 * Показывается при компиляции game.wasm и загрузке движка.
 */
class ProgressDialog(private val context: Context) {

    private var dialog: AlertDialog? = null
    private var progressBar: ProgressBar? = null
    private var messageText: TextView? = null
    private var onCancel: (() -> Unit)? = null

    fun show(onCancel: (() -> Unit)? = null) {
        this.onCancel = onCancel

        val view = LayoutInflater.from(context).inflate(R.layout.dialog_progress, null)
        progressBar = view.findViewById(R.id.progressBar)
        messageText = view.findViewById(R.id.messageText)

        val builder = AlertDialog.Builder(context)
            .setView(view)
            .setCancelable(false)

        dialog = builder.create()
        dialog?.show()
    }

    fun update(percent: Int, message: String) {
        progressBar?.progress = percent
        // Проценты показываем в тексте сообщения
        messageText?.text = "$percent% — $message"
    }

    fun setError(message: String) {
        progressBar?.visibility = View.GONE
        messageText?.text = "❌ $message"
        // Даём пользователю закрыть диалог через 5 секунд
        dialog?.setCancelable(true)
    }

    fun dismiss() {
        dialog?.dismiss()
        dialog = null
    }

    fun isShowing(): Boolean = dialog?.isShowing == true
}