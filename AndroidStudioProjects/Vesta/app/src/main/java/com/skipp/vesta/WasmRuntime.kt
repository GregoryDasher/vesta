package com.skipp.vesta

import android.util.Log
import android.view.Surface

object WasmRuntime {

    private const val TAG = "Muguet"
    private var loaded = false

    init {
        try {
            System.loadLibrary("muguet")
            loaded = true
            Log.i(TAG, "libmuguet.so успешно загружена")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Не удалось загрузить libmuguet.so: ${e.message}")
        }
    }

    fun isLoaded(): Boolean = loaded

    external fun nativeRun(gamePath: String): Int

    /** Передаёт Android Surface в Rust. Rust создаст из него ANativeWindow и wgpu::Surface */
    external fun nativeSurfaceCreated(surface: Surface, width: Int, height: Int)

    /** Сообщает Rust'у, что Surface уничтожен */
    external fun nativeSurfaceDestroyed()
}