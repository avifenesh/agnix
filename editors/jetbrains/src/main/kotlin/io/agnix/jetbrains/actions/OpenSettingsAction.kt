package io.agnix.jetbrains.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.options.ShowSettingsUtil
import io.agnix.jetbrains.AgnixIcons
import io.agnix.jetbrains.settings.AgnixSettingsConfigurable

/**
 * Action to open agnix settings.
 *
 * Opens the IDE settings dialog directly to the agnix configuration page.
 */
class OpenSettingsAction : AnAction(
    "Settings",
    "Open agnix settings",
    AgnixIcons.AGNIX
) {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project
        ShowSettingsUtil.getInstance().showSettingsDialog(
            project,
            AgnixSettingsConfigurable::class.java
        )
    }

    override fun update(e: AnActionEvent) {
        // Always available
        e.presentation.isEnabledAndVisible = true
    }
}
