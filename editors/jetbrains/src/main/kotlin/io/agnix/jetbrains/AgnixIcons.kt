package io.agnix.jetbrains

import com.intellij.openapi.util.IconLoader
import javax.swing.Icon

/**
 * Icon definitions for the agnix plugin.
 */
object AgnixIcons {

    /**
     * Main plugin icon (16x16).
     *
     * Used in tool windows, actions, and file types.
     */
    @JvmField
    val AGNIX: Icon = IconLoader.getIcon("/icons/agnix.svg", AgnixIcons::class.java)
}
