// @ts-check
import React, {useEffect, useRef, useState, useMemo, useCallback} from 'react';
import clsx from 'clsx';
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './playground.module.css';

const PRESETS = {
  'CLAUDE.md': `# Project Memory

Be helpful and accurate.

## Commands

\`\`\`bash
npm test
\`\`\`

<rules>
Always follow best practices.
`,
  'AGENTS.md': `# Project Instructions

Be concise and helpful.

Try to always write clean code.

## Tools

Use appropriate tools for the job.
`,
  'SKILL.md': `---
name: Review-Code
description: Reviews code for quality issues
---

# review-code

This skill reviews code and suggests improvements.
`,
  '.claude/agents/reviewer.md': `---
description: Reviews pull requests
model: gpt-4
permissionMode: bypassPermissions
tools:
  - Read
  - Grep
  - MagicTool
disallowedTools:
  - Read
---

Review code and suggest improvements.
`,
  // Intentionally unsafe hook config to demonstrate agnix validation rules
  '.claude/settings.json': `{
  "hooks": {
    "OnBeforeTool": [
      {
        "type": "command",
        "command": "rm -rf $DIR"
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash",
        "type": "command",
        "command": "echo checking"
      }
    ],
    "Stop": [
      {
        "matcher": "Bash",
        "type": "prompt",
        "prompt": "Summarize what happened"
      }
    ]
  }
}`,
  'plugin.json': `{
  "name": "",
  "description": "A useful plugin",
  "version": "1.0",
  "author": {
    "email": "dev@example.com"
  },
  "homepage": "not-a-url",
  "commands": "/absolute/path/commands"
}`,
  'mcp.json': `{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["server.js"]
    }
  }
}`,
};

const TOOLS = [
  {value: '', label: 'All tools'},
  {value: 'claude-code', label: 'Claude Code'},
  {value: 'cursor', label: 'Cursor'},
  {value: 'github-copilot', label: 'GitHub Copilot'},
  {value: 'codex', label: 'Codex CLI'},
  {value: 'cline', label: 'Cline'},
  {value: 'opencode', label: 'OpenCode'},
  {value: 'gemini-cli', label: 'Gemini CLI'},
  {value: 'roo-code', label: 'Roo Code'},
  {value: 'kiro', label: 'Kiro CLI'},
  {value: 'amp', label: 'amp'},
];

const DEFAULT_FILENAME = 'CLAUDE.md';

/**
 * @typedef {{
 *   start_byte: number,
 *   end_byte: number,
 *   replacement: string,
 *   description: string,
 *   safe: boolean,
 * }} WasmFix
 */

/**
 * @typedef {{
 *   level: string,
 *   rule: string,
 *   message: string,
 *   line: number,
 *   column: number,
 *   suggestion: string | null,
 *   assumption: string | null,
 *   fixes: WasmFix[],
 * }} WasmDiagnostic
 */

export default function PlaygroundPage() {
  return (
    <Layout
      title="Playground"
      description="Try agnix in your browser. Paste CLAUDE.md, SKILL.md, or any agent config and see validation results instantly."
    >
      <BrowserOnly fallback={<div role="status" aria-live="polite" className={styles.loading}><div className={styles.spinner} />Loading playground...</div>}>
        {() => <PlaygroundInner />}
      </BrowserOnly>
    </Layout>
  );
}

function useTheme() {
  const [theme, setTheme] = useState(() =>
    typeof document !== 'undefined'
      ? document.documentElement.getAttribute('data-theme') || 'light'
      : 'light',
  );
  useEffect(() => {
    const observer = new MutationObserver(() => {
      setTheme(
        document.documentElement.getAttribute('data-theme') || 'light',
      );
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    });
    return () => observer.disconnect();
  }, []);
  return theme;
}

function PlaygroundInner() {
  const colorMode = useTheme();

  const [filename, setFilename] = useState(DEFAULT_FILENAME);
  const [editorFilename, setEditorFilename] = useState(DEFAULT_FILENAME);
  const [tool, setTool] = useState('');
  const [content, setContent] = useState(PRESETS[DEFAULT_FILENAME]);
  const [diagnostics, setDiagnostics] = useState(
    /** @type {WasmDiagnostic[]} */ ([]),
  );
  const [fileType, setFileType] = useState('');
  const [wasmReady, setWasmReady] = useState(false);
  const [wasmError, setWasmError] = useState(/** @type {string | null} */ (null));
  const [validationError, setValidationError] = useState(/** @type {string | null} */ (null));
  const [loading, setLoading] = useState(true);
  const wasmRef = useRef(/** @type {any} */ (null));
  const editorRef = useRef(/** @type {HTMLDivElement | null} */ (null));
  const viewRef = useRef(/** @type {any} */ (null));
  const lintSetRef = useRef(/** @type {any} */ (null));
  const contentRef = useRef(content);

  // Keep contentRef in sync
  useEffect(() => {
    contentRef.current = content;
  }, [content]);

  // Debounce filename changes to avoid recreating editor on every keystroke
  useEffect(() => {
    const timer = setTimeout(() => setEditorFilename(filename), 500);
    return () => clearTimeout(timer);
  }, [filename]);

  // Convert a UTF-8 byte offset to a JavaScript string index.
  // Needed because Rust fix offsets are UTF-8 bytes, but CodeMirror uses JS string indices.
  const utf8ByteToStringIndex = useCallback((str, byteOffset) => {
    const encoder = new TextEncoder();
    let bytes = 0;
    for (let i = 0; i < str.length; i++) {
      const charBytes = encoder.encode(str[i]).length;
      if (bytes + charBytes > byteOffset) return i;
      bytes += charBytes;
    }
    return str.length;
  }, []);

  // Load WASM module
  useEffect(() => {
    let cancelled = false;
    async function loadWasm() {
      try {
        const wasm = await import('../../wasm/agnix_wasm.js');
        await wasm.default();
        if (!cancelled) {
          wasmRef.current = wasm;
          setWasmReady(true);
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setWasmError(String(err));
          setLoading(false);
        }
      }
    }
    loadWasm();
    return () => {
      cancelled = true;
    };
  }, []);

  // Initialize CodeMirror editor
  useEffect(() => {
    if (!editorRef.current) return;
    let view;
    async function setup() {
      const cm = await import('codemirror');
      const {EditorView} = await import('@codemirror/view');
      const {EditorState} = await import('@codemirror/state');
      const {oneDark} = await import('@codemirror/theme-one-dark');
      const {markdown} = await import('@codemirror/lang-markdown');
      const {json} = await import('@codemirror/lang-json');
      const {linter, lintGutter} = await import('@codemirror/lint');

      // Pick language mode based on filename
      const isJson = editorFilename.endsWith('.json');
      const lang = isJson ? json() : markdown();

      // External lint source: we push diagnostics from the validation effect
      /** @type {any[] | null} */
      let pendingDiagnostics = null;
      /** @type {((v: any) => void) | null} */
      let resolveNext = null;

      const agnixLinter = linter(
        () => new Promise((resolve) => {
          if (pendingDiagnostics !== null) {
            resolve(pendingDiagnostics);
            pendingDiagnostics = null;
          } else {
            resolveNext = resolve;
          }
        }),
        {delay: 0},
      );

      // Expose a setter so the validation effect can push diagnostics
      lintSetRef.current = (/** @type {any[]} */ diags) => {
        if (resolveNext) {
          resolveNext(diags);
          resolveNext = null;
        } else {
          pendingDiagnostics = diags;
        }
      };

      const extensions = [
        cm.basicSetup,
        lang,
        agnixLinter,
        lintGutter(),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            setContent(update.state.doc.toString());
          }
        }),
        EditorView.theme({
          '&': {height: '100%'},
          '.cm-scroller': {overflow: 'auto'},
        }),
      ];

      if (colorMode === 'dark') {
        extensions.push(oneDark);
      }

      const state = EditorState.create({
        doc: contentRef.current,
        extensions,
      });

      view = new EditorView({
        state,
        parent: editorRef.current,
      });

      viewRef.current = view;
    }

    setup();

    return () => {
      if (view) view.destroy();
    };
    // Recreate editor when theme, filename, or loading state changes.
    // `loading` is needed because editorRef.current is only in the DOM
    // after WASM loads (loading=false), so the effect must re-run then.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [colorMode, editorFilename, loading]);

  // Run validation (debounced) and push inline diagnostics to CodeMirror
  useEffect(() => {
    if (!wasmReady || !wasmRef.current) return;

    const timer = setTimeout(() => {
      try {
        const result = wasmRef.current.validate(
          filename,
          content,
          tool || undefined,
        );
        if (result && result.diagnostics) {
          setDiagnostics(result.diagnostics);
          setFileType(result.file_type || '');
          setValidationError(null);

          // Map diagnostics to CodeMirror lint format
          const v = viewRef.current;
          if (v && lintSetRef.current) {
            const doc = v.state.doc;
            const cmDiags = result.diagnostics.map((d) => {
              const line = Math.max(1, Math.min(d.line, doc.lines));
              const lineObj = doc.line(line);
              const from = lineObj.from + Math.max(0, (d.column || 1) - 1);
              const to = lineObj.to;
              const severity =
                d.level === 'error' ? 'error' :
                d.level === 'warning' ? 'warning' : 'info';
              return {
                from: Math.min(from, doc.length),
                to: Math.min(to, doc.length),
                severity,
                message: `[${d.rule}] ${d.message}`,
              };
            });
            lintSetRef.current(cmDiags);
            // Force the linter to re-check
            v.dispatch({});
          }
        }
      } catch (err) {
        setValidationError(String(err));
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [content, filename, tool, wasmReady]);

  // Counts by severity
  const counts = useMemo(() => {
    const c = {error: 0, warning: 0, info: 0};
    for (const d of diagnostics) {
      c[d.level] = (c[d.level] || 0) + 1;
    }
    return c;
  }, [diagnostics]);

  // Track which preset is active
  const activePreset = useMemo(() => {
    if (PRESETS[filename] && content === PRESETS[filename]) {
      return filename;
    }
    return null;
  }, [filename, content]);

  const handlePreset = useCallback(
    (name) => {
      setFilename(name);
      setEditorFilename(name);
      setContent(PRESETS[name]);
      if (viewRef.current) {
        const v = viewRef.current;
        v.dispatch({
          changes: {from: 0, to: v.state.doc.length, insert: PRESETS[name]},
        });
      }
    },
    [],
  );

  // Apply a single fix to the editor content
  const applyFix = useCallback(
    (/** @type {WasmFix} */ fix) => {
      const v = viewRef.current;
      if (!v) return;
      const text = v.state.doc.toString();
      const from = utf8ByteToStringIndex(text, fix.start_byte);
      const to = utf8ByteToStringIndex(text, fix.end_byte);
      v.dispatch({
        changes: {from, to, insert: fix.replacement},
      });
    },
    [utf8ByteToStringIndex],
  );

  // Apply all safe fixes (sorted in reverse order to preserve byte offsets)
  const applyAllFixes = useCallback(() => {
    const v = viewRef.current;
    if (!v) return;
    const text = v.state.doc.toString();
    const allFixes = diagnostics
      .flatMap((d) => d.fixes.filter((f) => f.safe))
      .sort((a, b) => b.start_byte - a.start_byte);
    if (allFixes.length === 0) return;
    const changes = allFixes.map((fix) => {
      const from = utf8ByteToStringIndex(text, fix.start_byte);
      const to = utf8ByteToStringIndex(text, fix.end_byte);
      return {from, to, insert: fix.replacement};
    });
    v.dispatch({changes});
  }, [diagnostics, utf8ByteToStringIndex]);

  const fixableCount = useMemo(() => {
    return diagnostics.reduce(
      (count, d) => count + d.fixes.filter((f) => f.safe).length,
      0,
    );
  }, [diagnostics]);

  return (
      <main className={styles.playground}>
        <div className="container">
          <div className={styles.header}>
            <h1 className={styles.title}>Playground</h1>
            <p className={styles.subtitle}>
              Validate agent configurations in your browser. No install
              required.
            </p>
          </div>

          {/* Controls */}
          <div className={styles.controls}>
            <div className={styles.controlGroup}>
              <label className={styles.label} htmlFor="pg-filename">
                Filename
              </label>
              <div className={styles.controlGroupRow}>
                <input
                  id="pg-filename"
                  className={styles.filenameInput}
                  type="text"
                  value={filename}
                  onChange={(e) => setFilename(e.target.value)}
                  placeholder="e.g. CLAUDE.md"
                  spellCheck={false}
                />
                {fileType && fileType !== 'Unknown' ? (
                  <span className={styles.fileTypeBadge}>{fileType}</span>
                ) : filename ? (
                  <span className={styles.fileTypeUnknown}>
                    Unknown type
                  </span>
                ) : null}
              </div>
            </div>

            <div className={styles.controlGroup}>
              <label className={styles.label} htmlFor="pg-tool">
                Tool
              </label>
              <select
                id="pg-tool"
                className={styles.toolSelect}
                value={tool}
                onChange={(e) => setTool(e.target.value)}
              >
                {TOOLS.map((t) => (
                  <option key={t.value} value={t.value}>
                    {t.label}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {/* Presets */}
          <div className={styles.presets}>
            <span className={styles.presetsLabel}>Try:</span>
            {Object.keys(PRESETS).map((name) => (
              <button
                key={name}
                className={clsx(
                  styles.presetButton,
                  activePreset === name && styles.presetButtonActive,
                )}
                onClick={() => handlePreset(name)}
                type="button"
              >
                {name}
              </button>
            ))}
          </div>

          {/* Editor + Results */}
          <div className={styles.layout}>
            <div className={styles.editorPane}>
              <div className={styles.editorLabel}>
                <span className={styles.editorLabelIcon}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <polyline points="16 18 22 12 16 6" />
                    <polyline points="8 6 2 12 8 18" />
                  </svg>
                </span>
                Editor
              </div>
              {loading ? (
                <div className={styles.loading}>
                  <div className={styles.spinner} />
                  Loading validation engine...
                </div>
              ) : wasmError ? (
                <div className={styles.error}>
                  <span className={styles.errorIcon}>
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <circle cx="12" cy="12" r="10" />
                      <line x1="15" y1="9" x2="9" y2="15" />
                      <line x1="9" y1="9" x2="15" y2="15" />
                    </svg>
                  </span>
                  Failed to load validation engine: {wasmError}
                </div>
              ) : (
                <div ref={editorRef} className={styles.editor} aria-label="Configuration editor" role="textbox" aria-multiline="true" />
              )}
            </div>

            <div className={styles.resultsPane}>
              <div className={styles.resultsHeader}>
                <span className={styles.resultsTitle}>
                  <span className={styles.resultsTitleIcon}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                    </svg>
                  </span>
                  Diagnostics
                </span>
                <span className={styles.resultsCounts}>
                  {counts.error > 0 && (
                    <span className={styles.countError}>
                      {counts.error} error
                      {counts.error !== 1 ? 's' : ''}
                    </span>
                  )}
                  {counts.warning > 0 && (
                    <span className={styles.countWarning}>
                      {counts.warning} warning
                      {counts.warning !== 1 ? 's' : ''}
                    </span>
                  )}
                  {counts.info > 0 && (
                    <span className={styles.countInfo}>
                      {counts.info} info
                    </span>
                  )}
                  {fixableCount > 0 && (
                    <button
                      className={styles.fixAllButton}
                      onClick={applyAllFixes}
                      type="button"
                      title={`Apply ${fixableCount} safe fix${fixableCount !== 1 ? 'es' : ''}`}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                      </svg>
                      Fix all ({fixableCount})
                    </button>
                  )}
                </span>
              </div>
              <div className={styles.diagnosticsList} aria-live="polite" aria-atomic="false">
                {validationError ? (
                  <div className={styles.emptyState}>
                    <div className={styles.emptyStateIcon} style={{background: 'hsla(0, 80%, 55%, 0.08)', color: '#ef4444'}}>
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <circle cx="12" cy="12" r="10" />
                        <line x1="15" y1="9" x2="9" y2="15" />
                        <line x1="9" y1="9" x2="15" y2="15" />
                      </svg>
                    </div>
                    <div className={styles.emptyStateText} style={{color: '#ef4444'}}>Validation error</div>
                    <p className={styles.emptyStateHint}>{validationError}</p>
                  </div>
                ) : diagnostics.length === 0 && wasmReady ? (
                  <div className={styles.emptyState}>
                    <div className={styles.emptyStateIcon}>
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                    </div>
                    <div className={styles.emptyStateText}>No issues found</div>
                    <p className={styles.emptyStateHint}>
                      Try one of the preset examples to see agnix in action.
                    </p>
                  </div>
                ) : (
                  diagnostics.map((d, i) => (
                    <div
                      key={`${d.rule}-${d.line}-${i}`}
                      className={styles.diagnostic}
                      data-level={d.level}
                    >
                      <div className={styles.diagnosticHeader}>
                        <span
                          className={styles.diagnosticLevel}
                          data-level={d.level}
                        >
                          {d.level}
                        </span>
                        <span className={styles.diagnosticRule}>
                          {d.rule}
                        </span>
                        <span className={styles.diagnosticLocation}>
                          line {d.line}
                          {d.column > 0 ? `:${d.column}` : ''}
                        </span>
                      </div>
                      <div className={styles.diagnosticMessage}>
                        {d.message}
                      </div>
                      {d.suggestion && (
                        <div className={styles.diagnosticSuggestion}>
                          {d.suggestion}
                        </div>
                      )}
                      {d.fixes.length > 0 && (
                        <div className={styles.diagnosticFixes}>
                          {d.fixes.map((fix, fi) => (
                            <button
                              key={fi}
                              className={styles.fixButton}
                              onClick={() => applyFix(fix)}
                              type="button"
                              title={fix.description}
                            >
                              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                                <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                              </svg>
                              {fix.description}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        </div>
      </main>
  );
}
