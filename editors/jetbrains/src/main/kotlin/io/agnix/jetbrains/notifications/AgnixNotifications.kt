package io.agnix.jetbrains.notifications

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.options.ShowSettingsUtil
import com.intellij.openapi.project.Project
import io.agnix.jetbrains.binary.AgnixBinaryDownloader
import io.agnix.jetbrains.binary.PlatformInfo
import io.agnix.jetbrains.settings.AgnixSettingsConfigurable

/**
 * Notification utilities for the agnix plugin.
 *
 * Provides methods for showing various notifications to the user.
 */
object AgnixNotifications {

    private const val NOTIFICATION_GROUP_ID = "agnix.notifications"

    /**
     * Notify that the LSP binary was not found.
     */
    fun notifyBinaryNotFound(project: Project) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp not found",
                "The agnix language server binary was not found. " +
                    "You can download it automatically or install manually.",
                NotificationType.WARNING
            )

        notification.addAction(object : AnAction("Download") {
            override fun actionPerformed(e: AnActionEvent) {
                val downloader = AgnixBinaryDownloader()
                downloader.downloadAsync(project) { path ->
                    if (path != null) {
                        notification.expire()
                    }
                }
            }
        })

        notification.addAction(object : AnAction("Settings") {
            override fun actionPerformed(e: AnActionEvent) {
                ShowSettingsUtil.getInstance().showSettingsDialog(
                    project,
                    AgnixSettingsConfigurable::class.java
                )
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that the current platform is not supported.
     */
    fun notifyPlatformNotSupported(project: Project) {
        val platformDesc = PlatformInfo.getPlatformDescription()

        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Platform not supported",
                "No pre-built agnix-lsp binary is available for $platformDesc. " +
                    "Please install manually: cargo install agnix-lsp",
                NotificationType.ERROR
            )
            .notify(project)
    }

    /**
     * Notify that the download was successful.
     */
    fun notifyDownloadSuccess(project: Project) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "agnix-lsp installed",
                "The agnix language server was successfully downloaded and installed.",
                NotificationType.INFORMATION
            )
            .notify(project)
    }

    /**
     * Notify that the download failed.
     */
    fun notifyDownloadFailed(project: Project, error: String) {
        val notification = NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Download failed",
                "Failed to download agnix-lsp: $error",
                NotificationType.ERROR
            )

        notification.addAction(object : AnAction("Retry") {
            override fun actionPerformed(e: AnActionEvent) {
                val downloader = AgnixBinaryDownloader()
                downloader.downloadAsync(project) { path ->
                    if (path != null) {
                        notification.expire()
                    }
                }
            }
        })

        notification.addAction(object : AnAction("Install Manually") {
            override fun actionPerformed(e: AnActionEvent) {
                // Open documentation URL
                com.intellij.ide.BrowserUtil.browse("https://github.com/avifenesh/agnix#installation")
                notification.expire()
            }
        })

        notification.notify(project)
    }

    /**
     * Notify that the LSP server encountered an error.
     */
    fun notifyServerError(project: Project, error: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(
                "Language server error",
                "agnix-lsp encountered an error: $error",
                NotificationType.ERROR
            )
            .notify(project)
    }

    /**
     * Show an informational notification.
     */
    fun notifyInfo(project: Project, title: String, content: String) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup(NOTIFICATION_GROUP_ID)
            .createNotification(title, content, NotificationType.INFORMATION)
            .notify(project)
    }
}
