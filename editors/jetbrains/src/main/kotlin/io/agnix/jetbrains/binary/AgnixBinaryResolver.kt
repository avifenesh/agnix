package io.agnix.jetbrains.binary

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.application.PathManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.extensions.PluginId
import java.io.File

/**
 * Resolves the location of the agnix-lsp binary.
 *
 * Searches for the binary in multiple locations:
 * 1. Plugin storage directory (downloaded binary)
 * 2. System PATH
 * 3. Common installation directories
 *
 * Uses caching to avoid repeated file system checks.
 */
object AgnixBinaryResolver {

    private val logger = Logger.getInstance(AgnixBinaryResolver::class.java)

    const val BINARY_NAME = "agnix-lsp"
    const val BINARY_NAME_WINDOWS = "agnix-lsp.exe"
    const val VERSION_MARKER_FILE = ".agnix-lsp-version"
    private const val PLUGIN_ID = "io.agnix.jetbrains"

    // Cache for resolved binary path - cleared when settings change or binary is downloaded
    @Volatile
    private var cachedBinaryPath: String? = null

    // Cache for PATH directories - lazy initialized
    private val pathDirectories: List<String> by lazy {
        System.getenv("PATH")?.split(File.pathSeparator) ?: emptyList()
    }

    /**
     * Clear the cached binary path. Call when settings change or after download.
     */
    fun clearCache() {
        cachedBinaryPath = null
    }

    /**
     * Get the plugin storage directory for downloaded binaries.
     */
    fun getStorageDirectory(): File {
        val pluginDir = PathManager.getPluginsPath()
        return File(pluginDir, "agnix/bin")
    }

    /**
     * Get the path to the downloaded binary, if it exists and its version matches the plugin.
     * Returns null if the binary is missing or its version marker doesn't match,
     * signalling that a re-download is needed.
     */
    fun getDownloadedBinaryPath(): String? {
        val binaryInfo = PlatformInfo.getBinaryInfo() ?: return null
        val binaryFile = File(getStorageDirectory(), binaryInfo.binaryName)

        if (!binaryFile.exists() || !binaryFile.canExecute()) {
            return null
        }

        val pluginVersion = getPluginVersion()
        val installedVersion = readVersionMarker()
        if (pluginVersion != null && installedVersion != pluginVersion) {
            logger.info(
                if (installedVersion != null)
                    "agnix-lsp version mismatch: binary=$installedVersion, plugin=$pluginVersion"
                else
                    "No version marker found for agnix-lsp, plugin=$pluginVersion"
            )
            return null
        }

        return binaryFile.absolutePath
    }

    /**
     * Get the current plugin version from the plugin descriptor.
     */
    fun getPluginVersion(): String? {
        return PluginManagerCore.getPlugin(PluginId.getId(PLUGIN_ID))?.version
    }

    /**
     * Read the version marker from the storage directory.
     */
    fun readVersionMarker(): String? {
        val markerFile = File(getStorageDirectory(), VERSION_MARKER_FILE)
        return try {
            if (markerFile.exists()) markerFile.readText().trim() else null
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Write the version marker to the storage directory.
     */
    fun writeVersionMarker(version: String) {
        val markerFile = File(getStorageDirectory(), VERSION_MARKER_FILE)
        try {
            markerFile.writeText(version)
        } catch (e: Exception) {
            logger.warn("Failed to write version marker", e)
        }
    }

    /**
     * Find agnix-lsp in the system PATH.
     */
    fun findInPath(): String? {
        val binaryName = getBinaryName()
        val extensions = if (PlatformInfo.getOS() == PlatformInfo.OS.WINDOWS) {
            listOf("", ".exe", ".cmd", ".bat")
        } else {
            listOf("")
        }

        for (dir in pathDirectories) {
            for (ext in extensions) {
                val file = File(dir, binaryName + ext)
                if (file.exists() && file.canExecute()) {
                    logger.info("Found agnix-lsp in PATH: ${file.absolutePath}")
                    return file.absolutePath
                }
            }
        }

        return null
    }

    /**
     * Find agnix-lsp in common installation directories.
     */
    fun findInCommonLocations(): String? {
        val binaryName = getBinaryName()
        val homeDir = System.getProperty("user.home")

        val locations = when (PlatformInfo.getOS()) {
            PlatformInfo.OS.MACOS -> listOf(
                "/usr/local/bin/$binaryName",
                "/opt/homebrew/bin/$binaryName",
                "$homeDir/.cargo/bin/$binaryName",
                "$homeDir/.local/bin/$binaryName"
            )
            PlatformInfo.OS.LINUX -> listOf(
                "/usr/local/bin/$binaryName",
                "/usr/bin/$binaryName",
                "$homeDir/.cargo/bin/$binaryName",
                "$homeDir/.local/bin/$binaryName"
            )
            PlatformInfo.OS.WINDOWS -> listOf(
                "$homeDir\\.cargo\\bin\\$binaryName",
                "C:\\Program Files\\agnix\\$binaryName"
            )
            PlatformInfo.OS.UNKNOWN -> emptyList()
        }

        for (location in locations) {
            val file = File(location)
            if (file.exists() && file.canExecute()) {
                logger.info("Found agnix-lsp at: ${file.absolutePath}")
                return file.absolutePath
            }
        }

        return null
    }

    /**
     * Resolve the binary path using all available methods with caching.
     *
     * Priority:
     * 1. Cached path (if still valid)
     * 2. Downloaded binary in plugin storage
     * 3. System PATH
     * 4. Common installation locations
     */
    fun resolve(): String? {
        // Check cache first
        cachedBinaryPath?.let { cached ->
            if (File(cached).exists()) {
                return cached
            }
            // Cache is stale, clear it
            cachedBinaryPath = null
        }

        // Check downloaded binary
        getDownloadedBinaryPath()?.let {
            cachedBinaryPath = it
            return it
        }

        // Check PATH
        findInPath()?.let {
            cachedBinaryPath = it
            return it
        }

        // Check common locations
        findInCommonLocations()?.let {
            cachedBinaryPath = it
            return it
        }

        logger.warn("agnix-lsp binary not found")
        return null
    }

    /**
     * Get the expected binary name for the current platform.
     */
    fun getBinaryName(): String {
        return if (PlatformInfo.getOS() == PlatformInfo.OS.WINDOWS) {
            BINARY_NAME_WINDOWS
        } else {
            BINARY_NAME
        }
    }

    /**
     * Check if a binary exists at the given path and is executable.
     */
    fun isValidBinary(path: String): Boolean {
        val file = File(path)
        return file.exists() && file.canExecute()
    }
}
