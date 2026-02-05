package io.agnix.jetbrains.settings

import com.intellij.openapi.options.Configurable
import io.agnix.jetbrains.binary.AgnixBinaryResolver
import javax.swing.JComponent

/**
 * Configurable for agnix settings in the IDE preferences.
 *
 * Accessible via: Settings/Preferences > Tools > agnix
 */
class AgnixSettingsConfigurable : Configurable {

    private var settingsComponent: AgnixSettingsComponent? = null

    override fun getDisplayName(): String = "agnix"

    override fun getPreferredFocusedComponent(): JComponent? {
        return settingsComponent?.getPreferredFocusedComponent()
    }

    override fun createComponent(): JComponent? {
        settingsComponent = AgnixSettingsComponent()
        return settingsComponent?.getPanel()
    }

    override fun isModified(): Boolean {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return false

        return component.enabled != settings.enabled ||
            component.lspPath != settings.lspPath ||
            component.autoDownload != settings.autoDownload ||
            component.traceLevel != settings.traceLevel ||
            component.codeLensEnabled != settings.codeLensEnabled
    }

    override fun apply() {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return

        val lspPathChanged = settings.lspPath != component.lspPath

        settings.enabled = component.enabled
        settings.lspPath = component.lspPath
        settings.autoDownload = component.autoDownload
        settings.traceLevel = component.traceLevel
        settings.codeLensEnabled = component.codeLensEnabled

        // Clear binary resolver cache if LSP path changed
        if (lspPathChanged) {
            AgnixBinaryResolver.clearCache()
        }
    }

    override fun reset() {
        val settings = AgnixSettings.getInstance()
        val component = settingsComponent ?: return

        component.enabled = settings.enabled
        component.lspPath = settings.lspPath
        component.autoDownload = settings.autoDownload
        component.traceLevel = settings.traceLevel
        component.codeLensEnabled = settings.codeLensEnabled
    }

    override fun disposeUIResources() {
        settingsComponent = null
    }
}
