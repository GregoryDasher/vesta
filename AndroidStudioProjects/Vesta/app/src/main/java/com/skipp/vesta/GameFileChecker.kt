package com.skipp.vesta

import java.io.File

object GameFilesChecker {

    data class CheckResult(
        val allPresent: Boolean,
        val displayName: String,
        val details: String
    )

    /**
     * Проверяет, что пользователь вручную скопировал содержимое
     * `mirror/playgta5.com/` в `Android/media/com.skipp.vesta/game/`.
     *
     * Ожидаемая структура:
     *   game/
     *   ├── data/manifest.json
     *   └── b/<build>/game.wasm
     */
    fun check(mediaDir: File): CheckResult {
        val gameDir = File(mediaDir, "game")
        val gamePath = gameDir.absolutePath

        if (!gameDir.exists() || !gameDir.isDirectory) {
            return CheckResult(
                allPresent = false,
                displayName = "game/ (пусто)",
                details = instructions(gamePath)
            )
        }

        // Ищем b/<build>/game.wasm
        val bDir = File(gameDir, "b")
        val buildDir = bDir.listFiles()?.firstOrNull { it.isDirectory }
        val gameWasm = buildDir?.let { File(it, "game.wasm") }
        val manifest = File(gameDir, "data/manifest.json")

        val found = mutableListOf<String>()
        val missing = mutableListOf<String>()

        if (gameWasm?.exists() == true) {
            found.add("  ✓ ${buildDir.name}/game.wasm (${formatSize(gameWasm.length())})")
        } else {
            missing.add("  ✗ b/<build>/game.wasm")
        }

        if (manifest.exists()) {
            found.add("  ✓ data/manifest.json (${formatSize(manifest.length())})")
        } else {
            missing.add("  ✗ data/manifest.json")
        }

        val details = buildString {
            append("Путь: $gamePath\n\n")
            if (found.isNotEmpty()) {
                append("Найдено:\n"); found.forEach { append(it).append("\n") }
            }
            if (missing.isNotEmpty()) {
                append("\nНе найдено:\n"); missing.forEach { append(it).append("\n") }
                append("\n"); append(instructions(gamePath))
            }
        }

        return CheckResult(
            allPresent = missing.isEmpty(),
            displayName = if (missing.isEmpty()) "game/ (ready)" else "game/ (incomplete)",
            details = details
        )
    }

    private fun instructions(gamePath: String): String = buildString {
        append("Как установить игру:\n\n")
        append("1. Подключите телефон к ПК по USB.\n")
        append("2. Откройте папку:\n")
        append("   Android/media/com.skipp.vesta/game/\n\n")
        append("3. Скопируйте туда СОДЕРЖИМОЕ папки:\n")
        append("   (ваша игра)/mirror/playgta5.com/\n\n")
        append("   Т.е. в game/ должны появиться:\n")
        append("   • b/           (папка)\n")
        append("   • data/        (папка)\n\n")
        append("Полный путь:\n$gamePath")
    }

    private fun formatSize(bytes: Long): String {
        if (bytes < 1024) return "$bytes B"
        val kb = bytes / 1024.0
        if (kb < 1024) return String.format("%.1f KB", kb)
        val mb = kb / 1024.0
        if (mb < 1024) return String.format("%.1f MB", mb)
        val gb = mb / 1024.0
        return String.format("%.2f GB", gb)
    }
}