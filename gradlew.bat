package com.skipp.vesta

import android.content.Context
import android.util.AttributeSet
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView

/**
 * SurfaceView, который отдаёт свою Surface в Rust (libmuguet.so).
 * Через JNI Rust получает ANativeWindow* и создаёт wgpu::Surface.
 */
class GameSurfaceView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = 0
) : SurfaceView(context, attrs, defStyleAttr), SurfaceHolder.Callback {

    private val tag = "Vesta.Surface"

    init {
        holder.addCallback(this)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        Log.i(tag, "surfaceCreated: ${holder.surface}")
        WasmRuntime.nativeSurfaceCreated(holder.surface, width, height)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        Log.i(tag, "surfaceChanged: ${width}x$height")
        // При изменении размеров пересоздаём Surface
        WasmRuntime.nativeSurfaceDestroyed()
        WasmRuntime.nativeSurfaceCreated(holder.surface, width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        Log.i(tag, "surfaceDestroyed")
        WasmRuntime.nativeSurfaceDestroyed()
    }
}