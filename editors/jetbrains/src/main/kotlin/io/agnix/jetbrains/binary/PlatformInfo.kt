package io.agnix.jetbrains.binary

import com.intellij.openapi.util.SystemInfo

/**
 * Platform detection utilities for agnix-lsp binary resolution.
 *
 * Detects the current OS and architecture to determine which binary asset to download.
 */
object PlatformInfo {

    /**
     * Supported operating systems.
     */
    enum class OS {
        WINDOWS,
        MACOS,
        LINUX,
        UNKNOWN
    }

    /**
     * Supported CPU architectures.
     */
    enum class Arch {
        X86_64,
        AARCH64,
        UNKNOWN
    }

    /**
     * Platform-specific binary information.
     */
    data class BinaryInfo(
        val assetName: String,
        val binaryName: String,
        val isArchive: Boolean = true
    )

    /**
     * Get the current operating system.
     */
    fun getOS(): OS {
        return when {
            SystemInfo.isWindows -> OS.WINDOWS
            SystemInfo.isMac -> OS.MACOS
            SystemInfo.isLinux -> OS.LINUX
            else -> OS.UNKNOWN
        }
    }

    /**
     * Get the current CPU architecture.
     */
    fun getArch(): Arch {
        val arch = System.getProperty("os.arch").lowercase()
        return when {
            arch == "amd64" || arch == "x86_64" -> Arch.X86_64
            arch == "aarch64" || arch == "arm64" -> Arch.AARCH64
            else -> Arch.UNKNOWN
        }
    }

    /**
     * Get the binary info for the current platform.
     *
     * Returns null if the platform is not supported.
     */
    fun getBinaryInfo(): BinaryInfo? {
        val os = getOS()
        val arch = getArch()

        return when (os) {
            OS.MACOS -> when (arch) {
                // macOS: Select native binary for each architecture
                // Rosetta only translates x86_64 on ARM, NOT ARM on Intel
                Arch.AARCH64 -> BinaryInfo(
                    assetName = "agnix-lsp-aarch64-apple-darwin.tar.gz",
                    binaryName = "agnix-lsp"
                )
                Arch.X86_64 -> BinaryInfo(
                    assetName = "agnix-lsp-x86_64-apple-darwin.tar.gz",
                    binaryName = "agnix-lsp"
                )
                else -> null
            }
            OS.LINUX -> when (arch) {
                Arch.X86_64 -> BinaryInfo(
                    assetName = "agnix-lsp-x86_64-unknown-linux-gnu.tar.gz",
                    binaryName = "agnix-lsp"
                )
                Arch.AARCH64 -> BinaryInfo(
                    assetName = "agnix-lsp-aarch64-unknown-linux-gnu.tar.gz",
                    binaryName = "agnix-lsp"
                )
                else -> null
            }
            OS.WINDOWS -> when (arch) {
                Arch.X86_64 -> BinaryInfo(
                    assetName = "agnix-lsp-x86_64-pc-windows-msvc.zip",
                    binaryName = "agnix-lsp.exe"
                )
                else -> null
            }
            OS.UNKNOWN -> null
        }
    }

    /**
     * Check if the current platform is supported.
     */
    fun isSupported(): Boolean = getBinaryInfo() != null

    /**
     * Get a human-readable platform description.
     */
    fun getPlatformDescription(): String {
        val os = when (getOS()) {
            OS.WINDOWS -> "Windows"
            OS.MACOS -> "macOS"
            OS.LINUX -> "Linux"
            OS.UNKNOWN -> "Unknown OS"
        }
        val arch = when (getArch()) {
            Arch.X86_64 -> "x86_64"
            Arch.AARCH64 -> "ARM64"
            Arch.UNKNOWN -> "Unknown Architecture"
        }
        return "$os ($arch)"
    }
}
