package com.music.bitchord.data

import android.content.Context
import android.content.SharedPreferences
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream

/**
 * Local store mapping playlist/album browse IDs to user-selected custom cover
 * images saved inside app-private storage (`filesDir/custom_covers`).
 */
object CustomCoverStore {
    private const val PREF_NAME = "bitchord_custom_covers"
    private const val KEY_COVERS = "covers_json"
    private const val MAX_DIM_PX = 1024

    private lateinit var prefs: SharedPreferences
    private lateinit var coverDir: File
    private val lock = Any()

    private val _covers = MutableStateFlow<Map<String, String>>(emptyMap())
    val covers: StateFlow<Map<String, String>> = _covers.asStateFlow()

    fun init(context: Context) {
        val app = context.applicationContext
        prefs = app.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        coverDir = File(app.filesDir, "custom_covers").apply { mkdirs() }
        val loaded = readStored()
        _covers.value = loaded
    }

    private fun keyVariants(browseId: String): List<String> {
        val clean = browseId.trim()
        if (clean.isEmpty()) return emptyList()
        val variants = linkedSetOf(clean)
        if (clean.startsWith("VL")) {
            val raw = clean.removePrefix("VL")
            if (raw.isNotEmpty()) variants.add(raw)
        } else if (!clean.contains(':')) {
            variants.add("VL$clean")
        }
        return variants.toList()
    }

    fun coverFor(browseId: String?): String? {
        val id = browseId?.trim()?.takeIf { it.isNotEmpty() } ?: return null
        val map = _covers.value
        for (k in keyVariants(id)) {
            val found = map[k]
            if (!found.isNullOrBlank()) return found
        }
        return null
    }

    fun hasCover(browseId: String?): Boolean = coverFor(browseId) != null

    suspend fun saveCover(context: Context, browseId: String, sourceUri: Uri): String? = withContext(Dispatchers.IO) {
        val cleanId = browseId.trim()
        if (cleanId.isEmpty() || !this@CustomCoverStore::prefs.isInitialized) return@withContext null

        val dir = coverDir.apply { mkdirs() }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        context.contentResolver.openInputStream(sourceUri)?.use { input ->
            BitmapFactory.decodeStream(input, null, bounds)
        }
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return@withContext null

        var sampleSize = 1
        while (bounds.outWidth / sampleSize > MAX_DIM_PX * 2 || bounds.outHeight / sampleSize > MAX_DIM_PX * 2) {
            sampleSize *= 2
        }

        val decodeOpts = BitmapFactory.Options().apply { inSampleSize = sampleSize }
        val decoded = context.contentResolver.openInputStream(sourceUri)?.use { input ->
            BitmapFactory.decodeStream(input, null, decodeOpts)
        } ?: return@withContext null

        val scaled = if (decoded.width > MAX_DIM_PX || decoded.height > MAX_DIM_PX) {
            val ratio = MAX_DIM_PX.toFloat() / maxOf(decoded.width, decoded.height).toFloat()
            val w = (decoded.width * ratio).toInt().coerceAtLeast(1)
            val h = (decoded.height * ratio).toInt().coerceAtLeast(1)
            Bitmap.createScaledBitmap(decoded, w, h, true).also {
                if (it !== decoded) decoded.recycle()
            }
        } else {
            decoded
        }

        val safeHash = cleanId.hashCode().toUInt().toString(16)
        val outFile = File(dir, "cover_${safeHash}_${System.currentTimeMillis()}.jpg")
        val savedOk = runCatching {
            FileOutputStream(outFile).use { fos ->
                scaled.compress(Bitmap.CompressFormat.JPEG, 90, fos)
            }
        }.isSuccess
        scaled.recycle()
        if (!savedOk || !outFile.exists()) {
            outFile.delete()
            return@withContext null
        }

        val fileUri = Uri.fromFile(outFile).toString()
        val oldUris = mutableListOf<String>()
        synchronized(lock) {
            val next = _covers.value.toMutableMap()
            for (k in keyVariants(cleanId)) {
                next.remove(k)?.let { oldUris.add(it) }
            }
            next[cleanId] = fileUri
            _covers.value = next
            persist(next)
        }
        oldUris.distinct().forEach { deleteOwnedFile(it) }
        fileUri
    }

    fun removeCover(browseId: String) {
        val cleanId = browseId.trim()
        if (cleanId.isEmpty() || !this::prefs.isInitialized) return
        val removed = mutableListOf<String>()
        synchronized(lock) {
            val next = _covers.value.toMutableMap()
            for (k in keyVariants(cleanId)) {
                next.remove(k)?.let { removed.add(it) }
            }
            if (removed.isEmpty()) return
            _covers.value = next
            persist(next)
        }
        removed.distinct().forEach { deleteOwnedFile(it) }
    }

    private fun deleteOwnedFile(uriStr: String) {
        runCatching {
            val parsed = Uri.parse(uriStr)
            if (parsed.scheme == "file") {
                val file = parsed.path?.let(::File) ?: return@runCatching
                if (this::coverDir.isInitialized && file.parentFile?.absolutePath == coverDir.absolutePath) {
                    file.delete()
                }
            }
        }
    }

    private fun readStored(): Map<String, String> {
        val raw = prefs.getString(KEY_COVERS, null) ?: return emptyMap()
        return runCatching {
            val obj = JSONObject(raw)
            val map = linkedMapOf<String, String>()
            val keys = obj.keys()
            while (keys.hasNext()) {
                val k = keys.next()
                val v = obj.optString(k)
                if (v.isNotBlank() && fileExistsFor(v)) {
                    map[k] = v
                }
            }
            if (map.size != obj.length()) persist(map)
            map
        }.getOrElse { emptyMap() }
    }

    private fun fileExistsFor(uriStr: String): Boolean = runCatching {
        val parsed = Uri.parse(uriStr)
        if (parsed.scheme == "file") parsed.path?.let { File(it).exists() } ?: false else true
    }.getOrDefault(false)

    private fun persist(map: Map<String, String>) {
        runCatching {
            val obj = JSONObject()
            for ((k, v) in map) obj.put(k, v)
            prefs.edit().putString(KEY_COVERS, obj.toString()).apply()
        }
    }
}
