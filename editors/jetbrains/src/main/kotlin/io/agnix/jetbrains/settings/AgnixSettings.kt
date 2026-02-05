package io.agnix.jetbrains.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.util.xmlb.XmlSerializerUtil

/**
 * Persistent settings for the agnix plugin.
 *
 * Settings are stored at the application level and shared across all projects.
 */
@State(
    name = "io.agnix.jetbrains.settings.AgnixSettings",
    storages = [Storage("agnix.xml")]
)
class AgnixSettings : PersistentStateComponent<AgnixSettings> {

    /**
     * Whether the plugin is enabled.
     */
    var enabled: Boolean = true

    /**
     * Custom path to the agnix-lsp binary.
     *
     * If empty, the plugin will search in the default locations.
     */
    var lspPath: String = ""

    /**
     * Whether to automatically download the LSP binary if not found.
     */
    var autoDownload: Boolean = true

    /**
     * Trace level for LSP communication (off, messages, verbose).
     */
    var traceLevel: TraceLevel = TraceLevel.OFF

    /**
     * Whether to show CodeLens annotations.
     */
    var codeLensEnabled: Boolean = true

    /**
     * Trace levels for LSP communication debugging.
     */
    enum class TraceLevel {
        OFF,
        MESSAGES,
        VERBOSE
    }

    override fun getState(): AgnixSettings = this

    override fun loadState(state: AgnixSettings) {
        XmlSerializerUtil.copyBean(state, this)
    }

    companion object {
        /**
         * Get the singleton instance of settings.
         */
        fun getInstance(): AgnixSettings {
            return ApplicationManager.getApplication().getService(AgnixSettings::class.java)
        }
    }
}
