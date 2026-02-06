import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';
import { downloadFile } from './download-file';
import { ClientLifecycleController } from './clientLifecycle';

const execAsync = promisify(exec);

let client: LanguageClient | undefined;
let lifecycleController: ClientLifecycleController<LanguageClient> | undefined;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;
let codeLensProvider: AgnixCodeLensProvider | undefined;
let diagnosticsTreeProvider: AgnixDiagnosticsTreeProvider | undefined;
let extensionContext: vscode.ExtensionContext;

const GITHUB_REPO = 'avifenesh/agnix';

interface PlatformInfo {
  asset: string;
  binary: string;
}

/**
 * LSP configuration structure sent to the language server.
 *
 * Maps VS Code settings to the Rust LintConfig structure.
 * Uses snake_case for Rust compatibility.
 */
interface LspConfig {
  severity?: string;
  target?: string;
  tools?: string[];
  locale?: string | null;
  rules?: {
    skills?: boolean;
    hooks?: boolean;
    agents?: boolean;
    memory?: boolean;
    plugins?: boolean;
    xml?: boolean;
    mcp?: boolean;
    imports?: boolean;
    cross_platform?: boolean;
    agents_md?: boolean;
    copilot?: boolean;
    cursor?: boolean;
    prompt_engineering?: boolean;
    disabled_rules?: string[];
  };
  versions?: {
    claude_code?: string | null;
    codex?: string | null;
    cursor?: string | null;
    copilot?: string | null;
  };
  specs?: {
    mcp_protocol?: string | null;
    agent_skills_spec?: string | null;
    agents_md_spec?: string | null;
  };
}

/**
 * Build LSP configuration from VS Code settings.
 *
 * Reads all agnix.* settings and maps them to the Rust LintConfig structure.
 * Handles the camelCase to snake_case conversion for Rust compatibility.
 *
 * @returns LspConfig object ready to send to the LSP server
 */
export function buildLspConfig(): LspConfig {
  const config = vscode.workspace.getConfiguration('agnix');

  // Helper to get user-set value (not schema default)
  const getUserValue = <T>(key: string): T | undefined => {
    const inspected = config.inspect<T>(key);
    if (!inspected) return undefined;
    // Priority: workspaceFolder > workspace > global > undefined (skip defaults)
    return inspected.workspaceFolderValue ?? inspected.workspaceValue ?? inspected.globalValue;
  };

  const result: LspConfig = {};

  // Only include fields explicitly set by user (preserves .agnix.toml defaults)
  const severity = getUserValue<string>('severity');
  if (severity !== undefined) result.severity = severity;

  const target = getUserValue<string>('target');
  if (target !== undefined) result.target = target;

  const tools = getUserValue<string[]>('tools');
  if (tools !== undefined) result.tools = tools;

  // Locale - support explicit null to revert to auto-detection
  const localeInspected = config.inspect<string | null>('locale');
  if (localeInspected && (localeInspected.workspaceFolderValue !== undefined ||
                          localeInspected.workspaceValue !== undefined ||
                          localeInspected.globalValue !== undefined)) {
    const localeValue = localeInspected.workspaceFolderValue ?? localeInspected.workspaceValue ?? localeInspected.globalValue;
    result.locale = localeValue; // null is valid (reverts to auto-detection)
  }

  // Rules - only include if user set them
  const rulesObj: any = {};
  let hasRules = false;

  const addRule = (key: string, field: string) => {
    const value = getUserValue<boolean>(key);
    if (value !== undefined) {
      rulesObj[field] = value;
      hasRules = true;
    }
  };

  addRule('rules.skills', 'skills');
  addRule('rules.hooks', 'hooks');
  addRule('rules.agents', 'agents');
  addRule('rules.memory', 'memory');
  addRule('rules.plugins', 'plugins');
  addRule('rules.xml', 'xml');
  addRule('rules.mcp', 'mcp');
  addRule('rules.imports', 'imports');
  addRule('rules.crossPlatform', 'cross_platform');
  addRule('rules.agentsMd', 'agents_md');
  addRule('rules.copilot', 'copilot');
  addRule('rules.cursor', 'cursor');
  addRule('rules.promptEngineering', 'prompt_engineering');

  const disabledRules = getUserValue<string[]>('rules.disabledRules');
  if (disabledRules !== undefined) {
    rulesObj.disabled_rules = disabledRules;
    hasRules = true;
  }

  if (hasRules) result.rules = rulesObj;

  // Versions - support explicit null to clear pins
  const versionsObj: any = {};
  let hasVersions = false;

  const addVersion = (key: string, field: string) => {
    const inspected = config.inspect<string | null>(key);
    if (inspected && (inspected.workspaceFolderValue !== undefined ||
                      inspected.workspaceValue !== undefined ||
                      inspected.globalValue !== undefined)) {
      const value = inspected.workspaceFolderValue ?? inspected.workspaceValue ?? inspected.globalValue;
      versionsObj[field] = value; // null is valid (clears pin)
      hasVersions = true;
    }
  };

  addVersion('versions.claudeCode', 'claude_code');
  addVersion('versions.codex', 'codex');
  addVersion('versions.cursor', 'cursor');
  addVersion('versions.copilot', 'copilot');

  if (hasVersions) result.versions = versionsObj;

  // Specs - support explicit null to clear pins
  const specsObj: any = {};
  let hasSpecs = false;

  const addSpec = (key: string, field: string) => {
    const inspected = config.inspect<string | null>(key);
    if (inspected && (inspected.workspaceFolderValue !== undefined ||
                      inspected.workspaceValue !== undefined ||
                      inspected.globalValue !== undefined)) {
      const value = inspected.workspaceFolderValue ?? inspected.workspaceValue ?? inspected.globalValue;
      specsObj[field] = value; // null is valid (clears pin)
      hasSpecs = true;
    }
  };

  addSpec('specs.mcpProtocol', 'mcp_protocol');
  addSpec('specs.agentSkills', 'agent_skills_spec');
  addSpec('specs.agentsMd', 'agents_md_spec');

  if (hasSpecs) result.specs = specsObj;

  return result;
}

const AGNIX_FILE_PATTERNS = [
  '**/SKILL.md',
  '**/CLAUDE.md',
  '**/CLAUDE.local.md',
  '**/AGENTS.md',
  '**/.claude/settings.json',
  '**/.claude/settings.local.json',
  '**/plugin.json',
  '**/*.mcp.json',
  '**/.github/copilot-instructions.md',
  '**/.github/instructions/*.instructions.md',
  '**/.cursor/rules/*.mdc',
];

const AGNIX_RULE_RE = /^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/;

function isAgnixDiagnostic(diagnostic: vscode.Diagnostic): boolean {
  const code = diagnostic.code?.toString() || '';
  return diagnostic.source === 'agnix' || AGNIX_RULE_RE.test(code);
}

function extractRuleIds(diagnostics: readonly vscode.Diagnostic[] | undefined): string[] {
  if (!diagnostics || diagnostics.length === 0) {
    return [];
  }
  const ids = diagnostics
    .map((d) => d.code?.toString() || '')
    .filter((code) => AGNIX_RULE_RE.test(code));
  return Array.from(new Set(ids));
}

function filterAgnixFixActions(
  actions: readonly vscode.CodeAction[] | undefined
): vscode.CodeAction[] {
  if (!actions) {
    return [];
  }
  return actions.filter((action) => {
    if (!action.edit) {
      return false;
    }
    const diagnostics = action.diagnostics || [];
    return diagnostics.some(isAgnixDiagnostic);
  });
}

/**
 * Get platform-specific download info for agnix-lsp.
 */
function getPlatformInfo(): PlatformInfo | null {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    if (arch === 'arm64') {
      return {
        asset: 'agnix-lsp-aarch64-apple-darwin.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    // x64 Mac can use ARM binary via Rosetta
    return {
      asset: 'agnix-lsp-aarch64-apple-darwin.tar.gz',
      binary: 'agnix-lsp',
    };
  } else if (platform === 'linux') {
    if (arch === 'x64') {
      return {
        asset: 'agnix-lsp-x86_64-unknown-linux-gnu.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    if (arch === 'arm64') {
      return {
        asset: 'agnix-lsp-aarch64-unknown-linux-gnu.tar.gz',
        binary: 'agnix-lsp',
      };
    }
    return null;
  } else if (platform === 'win32') {
    if (arch === 'x64') {
      return {
        asset: 'agnix-lsp-x86_64-pc-windows-msvc.zip',
        binary: 'agnix-lsp.exe',
      };
    }
    return null;
  }

  return null;
}

/**
 * Download and install agnix-lsp from GitHub releases.
 */
async function downloadAndInstallLsp(): Promise<string | null> {
  const platformInfo = getPlatformInfo();
  if (!platformInfo) {
    vscode.window.showErrorMessage(
      'No pre-built agnix-lsp available for your platform. Please install manually: cargo install agnix-lsp'
    );
    return null;
  }

  const releaseUrl = `https://github.com/${GITHUB_REPO}/releases/latest/download/${platformInfo.asset}`;

  // Create storage directory
  const storageUri = extensionContext.globalStorageUri;
  await vscode.workspace.fs.createDirectory(storageUri);

  const downloadPath = path.join(storageUri.fsPath, platformInfo.asset);
  const binaryPath = path.join(storageUri.fsPath, platformInfo.binary);

  try {
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'Installing agnix-lsp',
        cancellable: false,
      },
      async (progress) => {
        progress.report({ message: 'Downloading...' });
        outputChannel.appendLine(`Downloading from: ${releaseUrl}`);

        await downloadFile(releaseUrl, downloadPath);

        progress.report({ message: 'Extracting...' });
        outputChannel.appendLine(`Extracting to: ${storageUri.fsPath}`);

        if (process.platform === 'win32') {
          // PowerShell extraction for .zip
          await execAsync(
            `powershell -Command "Expand-Archive -Path '${downloadPath}' -DestinationPath '${storageUri.fsPath}' -Force"`,
            { timeout: 60000 }
          );
        } else {
          // tar extraction for .tar.gz
          await execAsync(
            `tar -xzf "${downloadPath}" -C "${storageUri.fsPath}"`,
            { timeout: 60000 }
          );

          // Verify binary exists before chmod
          if (!fs.existsSync(binaryPath)) {
            throw new Error(`Binary not found after extraction: ${binaryPath}`);
          }

          // Make executable - use fs.chmodSync instead of shell command to avoid injection risks
          fs.chmodSync(binaryPath, 0o755);
        }

        // Clean up archive
        try {
          fs.unlinkSync(downloadPath);
        } catch {
          // Error ignored during cleanup
        }
      }
    );

    // Verify binary exists
    if (fs.existsSync(binaryPath)) {
      outputChannel.appendLine(`agnix-lsp installed at: ${binaryPath}`);
      vscode.window.showInformationMessage('agnix-lsp installed successfully');
      return binaryPath;
    } else {
      throw new Error('Binary not found after extraction');
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Installation failed: ${message}`);
    vscode.window.showErrorMessage(`Failed to install agnix-lsp: ${message}`);
    return null;
  }
}

/**
 * Get the path to agnix-lsp, checking settings, PATH, and global storage.
 */
function findLspBinary(): string | null {
  const config = vscode.workspace.getConfiguration('agnix');
  const configuredPath = config.get<string>('lspPath', 'agnix-lsp');

  // Check configured path first
  if (checkLspExists(configuredPath)) {
    return configuredPath;
  }

  // Check if we have a downloaded binary in global storage
  const platformInfo = getPlatformInfo();
  if (platformInfo) {
    const storagePath = path.join(
      extensionContext.globalStorageUri.fsPath,
      platformInfo.binary
    );
    if (fs.existsSync(storagePath)) {
      return storagePath;
    }
  }

  return null;
}

export async function activate(
  context: vscode.ExtensionContext
): Promise<void> {
  extensionContext = context;
  outputChannel = vscode.window.createOutputChannel('agnix');
  context.subscriptions.push(outputChannel);

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.command = 'agnix.showOutput';
  context.subscriptions.push(statusBarItem);

  lifecycleController = new ClientLifecycleController<LanguageClient>({
    getClient: () => client,
    setClient: (nextClient) => {
      client = nextClient;
    },
    isClientActive: (runningClient: LanguageClient) => runningClient.isRunning(),
    startClient: () => startClientInternal(),
    stopClient: (runningClient: LanguageClient) => runningClient.stop(),
  });

  context.subscriptions.push(
    vscode.commands.registerCommand('agnix.restart', () => restartClient()),
    vscode.commands.registerCommand('agnix.showOutput', () =>
      outputChannel.show()
    ),
    vscode.commands.registerCommand('agnix.validateFile', () =>
      validateCurrentFile()
    ),
    vscode.commands.registerCommand('agnix.validateWorkspace', () =>
      validateWorkspace()
    ),
    vscode.commands.registerCommand('agnix.showRules', () => showRules()),
    vscode.commands.registerCommand('agnix.fixAll', () => fixAllInFile()),
    vscode.commands.registerCommand('agnix.previewFixes', () => previewFixes()),
    vscode.commands.registerCommand('agnix.fixAllSafe', () => fixAllSafeInFile()),
    vscode.commands.registerCommand('agnix.ignoreRule', (ruleId: string) => ignoreRule(ruleId)),
    vscode.commands.registerCommand('agnix.showRuleDoc', (ruleId: string) => showRuleDoc(ruleId))
  );

  // Register CodeLens provider
  codeLensProvider = new AgnixCodeLensProvider();
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider(
      [
        { scheme: 'file', language: 'markdown' },
        { scheme: 'file', language: 'skill-markdown' },
        { scheme: 'file', language: 'json' },
        { scheme: 'file', pattern: '**/*.mdc' },
      ],
      codeLensProvider
    )
  );

  // Update CodeLens when diagnostics change
  context.subscriptions.push(
    vscode.languages.onDidChangeDiagnostics((e) => {
      if (codeLensProvider) {
        codeLensProvider.refresh();
      }
      if (diagnosticsTreeProvider) {
        diagnosticsTreeProvider.refresh();
      }
    })
  );

  // Register Tree View for diagnostics
  diagnosticsTreeProvider = new AgnixDiagnosticsTreeProvider();
  context.subscriptions.push(
    vscode.window.createTreeView('agnixDiagnostics', {
      treeDataProvider: diagnosticsTreeProvider,
      showCollapseAll: true,
    })
  );

  // Register tree view commands
  context.subscriptions.push(
    vscode.commands.registerCommand('agnix.refreshDiagnostics', () => {
      diagnosticsTreeProvider?.refresh();
    }),
    vscode.commands.registerCommand('agnix.goToDiagnostic', (item: DiagnosticItem) => {
      if (item.diagnostic && item.uri) {
        vscode.window.showTextDocument(item.uri, {
          selection: item.diagnostic.range,
        });
      }
    })
  );

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration('agnix')) {
        const config = vscode.workspace.getConfiguration('agnix');
        if (!config.get<boolean>('enable', true)) {
          await stopClient();
        } else if (e.affectsConfiguration('agnix.lspPath')) {
          // LSP path changed, need full restart
          await restartClient();
        } else if (client && client.isRunning()) {
          // Other settings changed, send to server without restart
          const lspConfig = buildLspConfig();
          outputChannel.appendLine('Sending configuration update to LSP server');
          client.sendNotification('workspace/didChangeConfiguration', {
            settings: lspConfig,
          });
        } else {
          // Client not running but enable is true, start it
          await restartClient();
        }
      }
    })
  );

  const config = vscode.workspace.getConfiguration('agnix');
  if (config.get<boolean>('enable', true)) {
    await startClient();
  }
}

async function startClient(): Promise<void> {
  if (!lifecycleController) {
    return;
  }
  await lifecycleController.start();
  if (client && client.isRunning()) {
    updateStatusBar('ready', 'agnix');
  }
}

async function startClientInternal(): Promise<LanguageClient | undefined> {
  let lspPath = findLspBinary();

  if (!lspPath) {
    updateStatusBar('error', 'agnix-lsp not found');
    outputChannel.appendLine('agnix-lsp not found in PATH or settings');

    // Offer to download
    const choice = await vscode.window.showErrorMessage(
      'agnix-lsp not found. Would you like to download it automatically?',
      'Download',
      'Install Manually',
      'Open Settings'
    );

    if (choice === 'Download') {
      lspPath = await downloadAndInstallLsp();
      if (!lspPath) {
        return;
      }
    } else if (choice === 'Install Manually') {
      outputChannel.appendLine('');
      outputChannel.appendLine('To install agnix-lsp manually:');
      outputChannel.appendLine('  cargo install agnix-lsp');
      outputChannel.appendLine('');
      outputChannel.appendLine('Or via Homebrew (macOS/Linux):');
      outputChannel.appendLine('  brew tap avifenesh/agnix && brew install agnix');
      outputChannel.show();
      return;
    } else if (choice === 'Open Settings') {
      vscode.commands.executeCommand(
        'workbench.action.openSettings',
        'agnix.lspPath'
      );
      return;
    } else {
      return;
    }
  }

  outputChannel.appendLine(`Starting agnix-lsp from: ${lspPath}`);
  updateStatusBar('starting', 'Starting...');

  const serverOptions: ServerOptions = {
    run: {
      command: lspPath,
      transport: TransportKind.stdio,
    },
    debug: {
      command: lspPath,
      transport: TransportKind.stdio,
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'markdown' },
      { scheme: 'file', language: 'skill-markdown' },
      { scheme: 'file', language: 'json' },
      { scheme: 'file', pattern: '**/*.mdc' },
    ],
    synchronize: {
      fileEvents: AGNIX_FILE_PATTERNS.map((pattern) =>
        vscode.workspace.createFileSystemWatcher(pattern)
      ),
    },
    outputChannel,
    traceOutputChannel: outputChannel,
  };

  const nextClient = new LanguageClient(
    'agnix',
    'agnix Language Server',
    serverOptions,
    clientOptions
  );

  try {
    await nextClient.start();
    outputChannel.appendLine('agnix-lsp started successfully');

    // Send initial configuration to the LSP server
    const lspConfig = buildLspConfig();
    outputChannel.appendLine('Sending initial configuration to LSP server');
    nextClient.sendNotification('workspace/didChangeConfiguration', {
      settings: lspConfig,
    });
    return nextClient;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Failed to start agnix-lsp: ${message}`);
    updateStatusBar('error', 'agnix (error)');
    vscode.window.showErrorMessage(`Failed to start agnix-lsp: ${message}`);
    return undefined;
  }
}

async function stopClient(): Promise<void> {
  if (lifecycleController) {
    await lifecycleController.stop();
  }
  updateStatusBar('disabled', 'agnix (disabled)');
}

async function restartClient(): Promise<void> {
  outputChannel.appendLine('Restarting agnix-lsp...');
  if (!lifecycleController) {
    return;
  }
  await lifecycleController.restart();
  if (client && client.isRunning()) {
    updateStatusBar('ready', 'agnix');
  }
}

/**
 * Check if the LSP binary exists and is executable.
 * Uses safe filesystem checks instead of shell commands to prevent command injection.
 */
function checkLspExists(lspPath: string): boolean {
  // If it's a simple command name (no path separators), check PATH
  if (!lspPath.includes(path.sep) && !lspPath.includes('/')) {
    const pathEnv = process.env.PATH || '';
    const pathDirs = pathEnv.split(path.delimiter);
    const extensions =
      process.platform === 'win32' ? ['', '.exe', '.cmd', '.bat'] : [''];

    for (const dir of pathDirs) {
      for (const ext of extensions) {
        const fullPath = path.join(dir, lspPath + ext);
        try {
          fs.accessSync(fullPath, fs.constants.X_OK);
          return true;
        } catch {
          continue;
        }
      }
    }
    return false;
  }

  // Absolute or relative path - check directly
  try {
    const resolvedPath = path.resolve(lspPath);
    fs.accessSync(resolvedPath, fs.constants.X_OK);
    return true;
  } catch {
    // On Windows, try with .exe extension
    if (process.platform === 'win32' && !lspPath.endsWith('.exe')) {
      try {
        fs.accessSync(path.resolve(lspPath + '.exe'), fs.constants.X_OK);
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }
}

/**
 * Validate the currently open file by triggering LSP diagnostics refresh.
 */
async function validateCurrentFile(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage('No file is currently open');
    return;
  }

  if (!client) {
    vscode.window.showErrorMessage(
      'agnix language server is not running. Use "agnix: Restart Language Server" to start it.'
    );
    return;
  }

  // Force a document change to trigger re-validation
  const document = editor.document;
  outputChannel.appendLine(`Validating: ${document.fileName}`);

  // Touch the document to trigger diagnostics
  const edit = new vscode.WorkspaceEdit();
  const lastLine = document.lineAt(document.lineCount - 1);
  edit.insert(document.uri, lastLine.range.end, '');
  await vscode.workspace.applyEdit(edit);

  vscode.window.showInformationMessage(
    `Validating ${path.basename(document.fileName)}...`
  );
}

/**
 * Validate all agent config files in the workspace.
 */
async function validateWorkspace(): Promise<void> {
  if (!client) {
    vscode.window.showErrorMessage(
      'agnix language server is not running. Use "agnix: Restart Language Server" to start it.'
    );
    return;
  }

  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders) {
    vscode.window.showWarningMessage('No workspace folder is open');
    return;
  }

  outputChannel.appendLine('Validating workspace...');

  // Find all agnix files and open them to trigger validation
  const patterns = AGNIX_FILE_PATTERNS.map((p) => new vscode.RelativePattern(workspaceFolders[0], p));

  let fileCount = 0;
  for (const pattern of patterns) {
    const files = await vscode.workspace.findFiles(pattern, '**/node_modules/**', 100);
    fileCount += files.length;

    for (const file of files) {
      // Open document to trigger LSP validation
      await vscode.workspace.openTextDocument(file);
    }
  }

  outputChannel.appendLine(`Found ${fileCount} agent config files`);
  vscode.window.showInformationMessage(
    `Validating ${fileCount} agent config files. Check Problems panel for results.`
  );

  // Focus problems panel
  vscode.commands.executeCommand('workbench.panel.markers.view.focus');
}

/**
 * Show all available validation rules.
 */
async function showRules(): Promise<void> {
  const rules = [
    { category: 'Agent Skills (AS-*)', count: 15, description: 'SKILL.md validation' },
    { category: 'Claude Code Skills (CC-SK-*)', count: 8, description: 'Claude-specific skill rules' },
    { category: 'Claude Code Hooks (CC-HK-*)', count: 12, description: 'Hooks configuration' },
    { category: 'Claude Code Agents (CC-AG-*)', count: 7, description: 'Agent definitions' },
    { category: 'Claude Code Plugins (CC-PL-*)', count: 6, description: 'Plugin manifests' },
    { category: 'Prompt Engineering (PE-*)', count: 10, description: 'Prompt quality' },
    { category: 'MCP (MCP-*)', count: 8, description: 'Model Context Protocol' },
    { category: 'Memory Files (AGM-*)', count: 8, description: 'AGENTS.md validation' },
    { category: 'GitHub Copilot (COP-*)', count: 6, description: 'Copilot instructions' },
    { category: 'Cursor (CUR-*)', count: 6, description: 'Cursor rules' },
    { category: 'XML (XML-*)', count: 4, description: 'XML tag formatting' },
    { category: 'Cross-Platform (XP-*)', count: 10, description: 'Multi-tool compatibility' },
  ];

  const items = rules.map((r) => ({
    label: r.category,
    description: `${r.count} rules`,
    detail: r.description,
  }));

  const selected = await vscode.window.showQuickPick(items, {
    title: 'agnix Validation Rules (100 total)',
    placeHolder: 'Select category to learn more',
  });

  if (selected) {
    // Open documentation
    vscode.env.openExternal(
      vscode.Uri.parse(
        'https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md'
      )
    );
  }
}

/**
 * Apply all available fixes in the current file.
 */
async function fixAllInFile(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage('No file is currently open');
    return;
  }

  if (!client) {
    vscode.window.showErrorMessage(
      'agnix language server is not running. Use "agnix: Restart Language Server" to start it.'
    );
    return;
  }

  // Get all code actions for the document
  const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
  const agnixDiagnostics = diagnostics.filter(isAgnixDiagnostic);

  if (agnixDiagnostics.length === 0) {
    vscode.window.showInformationMessage('No agnix issues found in this file');
    return;
  }

  // Execute source.fixAll code action
  const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
    'vscode.executeCodeActionProvider',
    editor.document.uri,
    new vscode.Range(0, 0, editor.document.lineCount, 0),
    vscode.CodeActionKind.QuickFix.value
  );

  const agnixActions = filterAgnixFixActions(actions);

  if (agnixActions.length === 0) {
    vscode.window.showInformationMessage(
      `No automatic agnix fixes available (${agnixDiagnostics.length} issue${agnixDiagnostics.length === 1 ? '' : 's'})`
    );
    return;
  }

  let applied = 0;
  let safeApplied = 0;
  for (const action of agnixActions) {
    if (action.edit) {
      await vscode.workspace.applyEdit(action.edit);
      applied++;
      if (action.isPreferred === true) {
        safeApplied++;
      }
    }
  }

  if (applied > 0) {
    const reviewApplied = applied - safeApplied;
    vscode.window.showInformationMessage(
      `Applied ${applied} agnix fixes (${safeApplied} safe, ${reviewApplied} review)`
    );
  } else {
    vscode.window.showInformationMessage(
      'No automatic agnix fixes could be applied'
    );
  }
}

/**
 * Preview all available fixes before applying them.
 * Shows a quick pick with fix details and confidence level.
 */
async function previewFixes(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage('No file is currently open');
    return;
  }

  if (!client) {
    vscode.window.showErrorMessage(
      'agnix language server is not running. Use "agnix: Restart Language Server" to start it.'
    );
    return;
  }

  const document = editor.document;
  const diagnostics = vscode.languages.getDiagnostics(document.uri);
  const agnixDiagnostics = diagnostics.filter(isAgnixDiagnostic);
  if (agnixDiagnostics.length === 0) {
    vscode.window.showInformationMessage('No agnix issues found in this file');
    return;
  }

  const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
    'vscode.executeCodeActionProvider',
    document.uri,
    new vscode.Range(0, 0, document.lineCount, 0),
    vscode.CodeActionKind.QuickFix.value
  );

  const agnixActions = filterAgnixFixActions(actions);

  if (agnixActions.length === 0) {
    vscode.window.showInformationMessage(
      `No automatic agnix fixes available (${agnixDiagnostics.length} issue${agnixDiagnostics.length === 1 ? '' : 's'})`
    );
    return;
  }

  // Build quick pick items with confidence indicators
  const items: (vscode.QuickPickItem & { action: vscode.CodeAction })[] = agnixActions
    .map((action) => {
      const isSafe = action.isPreferred === true;
      const confidence = isSafe ? '$(check) Safe' : '$(warning) Review';
      const ruleIds = extractRuleIds(action.diagnostics);
      const rulesText = ruleIds.length > 0 ? `Rules: ${ruleIds.join(', ')}` : 'Rules unavailable';
      return {
        label: `${confidence}  ${action.title}`,
        description: getEditSummary(action.edit!, document),
        detail: `${isSafe ? 'Safe to apply automatically' : 'Review before applying'} • ${rulesText}`,
        action,
      };
    });

  if (items.length === 0) {
    vscode.window.showInformationMessage('No automatic agnix fixes available for this file');
    return;
  }

  // Add "Apply All" options at the top
  const applyAllItem = {
    label: '$(checklist) Apply All Fixes',
    description: `${items.length} fixes`,
    detail: 'Apply all available fixes at once',
    action: null as unknown as vscode.CodeAction,
  };

  const safeCount = items.filter((i) => i.action.isPreferred === true).length;
  const applyAllSafeItem: vscode.QuickPickItem & { action: vscode.CodeAction } = {
    label: '$(shield) Apply All Safe Fixes',
    description: `${safeCount} safe fixes`,
    detail: 'Only apply fixes marked as safe',
    picked: safeCount === items.length,
    action: null as unknown as vscode.CodeAction,
  };

  const allItems: (vscode.QuickPickItem & { action: vscode.CodeAction })[] = [applyAllItem];
  if (safeCount > 0) {
    allItems.push(applyAllSafeItem);
  }
  allItems.push({ label: '', kind: vscode.QuickPickItemKind.Separator, action: null as unknown as vscode.CodeAction });
  allItems.push(...items);

  const selected = await vscode.window.showQuickPick(allItems, {
    title: `agnix Fixes Preview (${items.length} available)`,
    placeHolder: 'Select a fix to preview or apply',
    matchOnDescription: true,
    matchOnDetail: true,
  });

  if (!selected) {
    return;
  }

  if (selected.label === '$(checklist) Apply All Fixes') {
    await applyAllFixes(items.map((i) => i.action));
    return;
  }

  if (safeCount > 0 && selected.label === '$(shield) Apply All Safe Fixes') {
    const safeActions = items.filter((i) => i.action.isPreferred === true).map((i) => i.action);
    await applyAllFixes(safeActions);
    return;
  }

  // Show diff preview for single fix
  await showFixPreview(document, selected.action);
}

/**
 * Get a summary of what an edit will change.
 */
function getEditSummary(edit: vscode.WorkspaceEdit, document: vscode.TextDocument): string {
  const changes = edit.get(document.uri);
  if (!changes || changes.length === 0) {
    return '';
  }

  if (changes.length === 1) {
    const change = changes[0];
    const lineNum = change.range.start.line + 1;
    if (change.newText === '') {
      return `Line ${lineNum}: delete text`;
    }
    if (change.range.isEmpty) {
      return `Line ${lineNum}: insert text`;
    }
    return `Line ${lineNum}: replace text`;
  }

  let inserts = 0;
  let deletes = 0;
  let replaces = 0;

  for (const change of changes) {
    if (change.newText === '') {
      deletes++;
    } else if (change.range.isEmpty) {
      inserts++;
    } else {
      replaces++;
    }
  }

  const parts: string[] = [];
  if (replaces > 0) parts.push(`${replaces} replace${replaces > 1 ? 's' : ''}`);
  if (inserts > 0) parts.push(`${inserts} insert${inserts > 1 ? 's' : ''}`);
  if (deletes > 0) parts.push(`${deletes} delete${deletes > 1 ? 's' : ''}`);

  if (parts.length === 0) {
    return `${changes.length} changes`;
  }
  return `${changes.length} changes (${parts.join(', ')})`;
}

/**
 * Show a diff preview for a single fix.
 */
async function showFixPreview(
  document: vscode.TextDocument,
  action: vscode.CodeAction
): Promise<void> {
  if (!action.edit) {
    return;
  }

  const originalContent = document.getText();
  const changes = action.edit.get(document.uri);

  if (!changes || changes.length === 0) {
    return;
  }

  // Apply changes to create preview content
  let previewContent = originalContent;
  // Sort changes in reverse order to apply from end to start
  const sortedChanges = [...changes].sort(
    (a, b) => b.range.start.compareTo(a.range.start)
  );

  for (const change of sortedChanges) {
    const startOffset = document.offsetAt(change.range.start);
    const endOffset = document.offsetAt(change.range.end);
    previewContent =
      previewContent.substring(0, startOffset) +
      change.newText +
      previewContent.substring(endOffset);
  }

  // Create virtual documents for diff
  const originalUri = vscode.Uri.parse(
    `agnix-preview:${document.uri.path}?original`
  );
  const previewUri = vscode.Uri.parse(
    `agnix-preview:${document.uri.path}?preview`
  );

  // Register content provider for virtual documents
  const provider = new (class implements vscode.TextDocumentContentProvider {
    provideTextDocumentContent(uri: vscode.Uri): string {
      if (uri.query === 'original') {
        return originalContent;
      }
      return previewContent;
    }
  })();

  const registration = vscode.workspace.registerTextDocumentContentProvider(
    'agnix-preview',
    provider
  );

  try {
    const changedLines = new Set<number>();
    for (const change of changes) {
      const startLine = change.range.start.line;
      const endLine = Math.max(change.range.end.line, startLine);
      for (let line = startLine; line <= endLine; line++) {
        changedLines.add(line);
      }
    }

    // Show diff
    await vscode.commands.executeCommand(
      'vscode.diff',
      originalUri,
      previewUri,
      `${path.basename(document.fileName)}: Fix Preview (${changedLines.size} changed line${changedLines.size === 1 ? '' : 's'}) - ${action.title}`,
      { preview: true }
    );

    // Ask user to apply
    const isSafe = action.isPreferred === true;
    const confidence = isSafe ? 'Safe fix' : 'Review recommended';
    const ruleIds = extractRuleIds(action.diagnostics);
    const rulesText = ruleIds.length > 0 ? ` (${ruleIds.join(', ')})` : '';

    const choice = await vscode.window.showInformationMessage(
      `${confidence}${rulesText}: ${action.title}`,
      { modal: false },
      'Apply Fix',
      'Open Rule Doc',
      'Cancel'
    );

    if (choice === 'Apply Fix') {
      await vscode.workspace.applyEdit(action.edit);
      vscode.window.showInformationMessage('Fix applied');
    } else if (choice === 'Open Rule Doc') {
      if (ruleIds.length === 1) {
        await showRuleDoc(ruleIds[0]);
      } else if (ruleIds.length > 1) {
        vscode.window.showInformationMessage(
          `This fix maps to multiple rules: ${ruleIds.join(', ')}`
        );
      } else {
        vscode.window.showInformationMessage(
          'Rule documentation is unavailable for this fix'
        );
      }
    }
  } finally {
    registration.dispose();
  }
}

/**
 * Apply multiple fixes.
 */
async function applyAllFixes(actions: vscode.CodeAction[]): Promise<void> {
  let fixCount = 0;
  for (const action of actions) {
    if (action.edit) {
      await vscode.workspace.applyEdit(action.edit);
      fixCount++;
    }
  }

  if (fixCount > 0) {
    vscode.window.showInformationMessage(`Applied ${fixCount} fixes`);
  } else {
    vscode.window.showInformationMessage('No fixes could be applied');
  }
}

/**
 * Apply only safe fixes in the current file.
 */
async function fixAllSafeInFile(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage('No file is currently open');
    return;
  }

  if (!client) {
    vscode.window.showErrorMessage(
      'agnix language server is not running. Use "agnix: Restart Language Server" to start it.'
    );
    return;
  }

  const diagnostics = vscode.languages.getDiagnostics(editor.document.uri);
  const agnixDiagnostics = diagnostics.filter(isAgnixDiagnostic);
  if (agnixDiagnostics.length === 0) {
    vscode.window.showInformationMessage('No agnix issues found in this file');
    return;
  }

  const actions = await vscode.commands.executeCommand<vscode.CodeAction[]>(
    'vscode.executeCodeActionProvider',
    editor.document.uri,
    new vscode.Range(0, 0, editor.document.lineCount, 0),
    vscode.CodeActionKind.QuickFix.value
  );

  const agnixActions = filterAgnixFixActions(actions);
  if (agnixActions.length === 0) {
    vscode.window.showInformationMessage(
      `No automatic agnix fixes available (${agnixDiagnostics.length} issue${agnixDiagnostics.length === 1 ? '' : 's'})`
    );
    return;
  }

  // Filter to only safe fixes (isPreferred = true)
  const safeActions = agnixActions.filter((a) => a.isPreferred === true && a.edit);

  if (safeActions.length === 0) {
    vscode.window.showInformationMessage(
      `No safe fixes available (${agnixActions.length} review fix${agnixActions.length === 1 ? '' : 'es'} available). Use "Preview Fixes" to review.`
    );
    return;
  }

  let fixCount = 0;
  for (const action of safeActions) {
    if (action.edit) {
      await vscode.workspace.applyEdit(action.edit);
      fixCount++;
    }
  }

  const skipped = agnixActions.length - fixCount;
  if (skipped > 0) {
    vscode.window.showInformationMessage(
      `Applied ${fixCount} safe agnix fixes (${skipped} review fixes skipped)`
    );
  } else {
    vscode.window.showInformationMessage(`Applied ${fixCount} safe agnix fixes`);
  }
}

/**
 * CodeLens provider for agnix diagnostics.
 * Shows rule info inline above lines with issues.
 */
class AgnixCodeLensProvider implements vscode.CodeLensProvider {
  private _onDidChangeCodeLenses = new vscode.EventEmitter<void>();
  public readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

  refresh(): void {
    this._onDidChangeCodeLenses.fire();
  }

  provideCodeLenses(
    document: vscode.TextDocument,
    _token: vscode.CancellationToken
  ): vscode.CodeLens[] {
    const config = vscode.workspace.getConfiguration('agnix');
    if (!config.get<boolean>('codeLens.enable', true)) {
      return [];
    }

    const diagnostics = vscode.languages.getDiagnostics(document.uri);
    const agnixDiagnostics = diagnostics.filter(
      (d) =>
        d.source === 'agnix' ||
        d.code?.toString().match(/^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/)
    );

    if (agnixDiagnostics.length === 0) {
      return [];
    }

    // Group diagnostics by line
    const byLine = new Map<number, vscode.Diagnostic[]>();
    for (const diag of agnixDiagnostics) {
      const line = diag.range.start.line;
      if (!byLine.has(line)) {
        byLine.set(line, []);
      }
      byLine.get(line)!.push(diag);
    }

    const codeLenses: vscode.CodeLens[] = [];

    for (const [line, diags] of byLine) {
      const range = new vscode.Range(line, 0, line, 0);

      // Create summary CodeLens
      const errors = diags.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Error
      ).length;
      const warnings = diags.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Warning
      ).length;

      const parts: string[] = [];
      if (errors > 0) parts.push(`${errors} error${errors > 1 ? 's' : ''}`);
      if (warnings > 0) parts.push(`${warnings} warning${warnings > 1 ? 's' : ''}`);

      const ruleIds = diags.map((d) => d.code?.toString() || '').filter(Boolean);
      const rulesText = ruleIds.length <= 2 ? ruleIds.join(', ') : `${ruleIds.length} rules`;

      codeLenses.push(
        new vscode.CodeLens(range, {
          title: `$(warning) ${parts.join(', ')} (${rulesText})`,
          command: 'agnix.previewFixes',
          tooltip: `Click to preview fixes for: ${ruleIds.join(', ')}`,
        })
      );

      // Add individual rule CodeLenses for quick actions
      for (const diag of diags.slice(0, 3)) {
        const ruleId = diag.code?.toString() || '';
        if (ruleId) {
          codeLenses.push(
            new vscode.CodeLens(range, {
              title: `$(info) ${ruleId}`,
              command: 'agnix.showRuleDoc',
              arguments: [ruleId],
              tooltip: `${diag.message} - Click for rule documentation`,
            })
          );
        }
      }
    }

    return codeLenses;
  }
}

/**
 * Tree item for diagnostics tree view.
 */
class DiagnosticItem extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState,
    public readonly uri?: vscode.Uri,
    public readonly diagnostic?: vscode.Diagnostic,
    public readonly children?: DiagnosticItem[]
  ) {
    super(label, collapsibleState);

    if (diagnostic && uri) {
      this.description = `Line ${diagnostic.range.start.line + 1}`;
      this.tooltip = diagnostic.message;
      this.command = {
        command: 'agnix.goToDiagnostic',
        title: 'Go to Diagnostic',
        arguments: [this],
      };

      // Set icon based on severity
      if (diagnostic.severity === vscode.DiagnosticSeverity.Error) {
        this.iconPath = new vscode.ThemeIcon('error', new vscode.ThemeColor('errorForeground'));
      } else if (diagnostic.severity === vscode.DiagnosticSeverity.Warning) {
        this.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('editorWarning.foreground'));
      } else {
        this.iconPath = new vscode.ThemeIcon('info', new vscode.ThemeColor('editorInfo.foreground'));
      }
    }
  }
}

/**
 * Tree data provider for agnix diagnostics.
 * Shows diagnostics organized by file.
 */
class AgnixDiagnosticsTreeProvider implements vscode.TreeDataProvider<DiagnosticItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<DiagnosticItem | undefined>();
  public readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: DiagnosticItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: DiagnosticItem): DiagnosticItem[] {
    if (element) {
      return element.children || [];
    }

    // Root level: show files with diagnostics
    const allDiagnostics = vscode.languages.getDiagnostics();
    const fileItems: DiagnosticItem[] = [];

    for (const [uri, diagnostics] of allDiagnostics) {
      const agnixDiagnostics = diagnostics.filter(
        (d) =>
          d.source === 'agnix' ||
          d.code?.toString().match(/^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/)
      );

      if (agnixDiagnostics.length === 0) {
        continue;
      }

      const errors = agnixDiagnostics.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Error
      ).length;
      const warnings = agnixDiagnostics.filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Warning
      ).length;

      // Create children for this file
      const children = agnixDiagnostics.map((diag) => {
        const ruleId = diag.code?.toString() || '';
        return new DiagnosticItem(
          `${ruleId}: ${diag.message}`,
          vscode.TreeItemCollapsibleState.None,
          uri,
          diag
        );
      });

      const fileName = path.basename(uri.fsPath);
      const counts: string[] = [];
      if (errors > 0) counts.push(`${errors} error${errors > 1 ? 's' : ''}`);
      if (warnings > 0) counts.push(`${warnings} warning${warnings > 1 ? 's' : ''}`);

      const fileItem = new DiagnosticItem(
        fileName,
        vscode.TreeItemCollapsibleState.Expanded,
        uri,
        undefined,
        children
      );
      fileItem.description = counts.join(', ');
      fileItem.iconPath = vscode.ThemeIcon.File;
      fileItem.resourceUri = uri;

      fileItems.push(fileItem);
    }

    if (fileItems.length === 0) {
      const noIssues = new DiagnosticItem(
        'No issues found',
        vscode.TreeItemCollapsibleState.None
      );
      noIssues.iconPath = new vscode.ThemeIcon('check', new vscode.ThemeColor('testing.iconPassed'));
      return [noIssues];
    }

    return fileItems;
  }
}

/**
 * Show documentation for a specific rule.
 */
async function showRuleDoc(ruleId: string): Promise<void> {
  const ruleCategories: Record<string, string> = {
    AS: 'agent-skills',
    'CC-SK': 'claude-skills',
    'CC-HK': 'claude-hooks',
    'CC-AG': 'claude-agents',
    'CC-PL': 'claude-plugins',
    'CC-MEM': 'claude-memory',
    PE: 'prompt-engineering',
    MCP: 'mcp',
    AGM: 'agents-md',
    COP: 'copilot',
    CUR: 'cursor',
    XML: 'xml',
    XP: 'cross-platform',
  };

  // Find category for rule
  let category = 'agent-skills';
  for (const [prefix, cat] of Object.entries(ruleCategories)) {
    if (ruleId.startsWith(prefix)) {
      category = cat;
      break;
    }
  }

  const url = `https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#${ruleId.toLowerCase()}`;
  vscode.env.openExternal(vscode.Uri.parse(url));
}

/**
 * Ignore a rule (add to disabled_rules in .agnix.toml).
 */
async function ignoreRule(ruleId: string): Promise<void> {
  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders) {
    vscode.window.showWarningMessage('No workspace folder open');
    return;
  }

  const configPath = path.join(workspaceFolders[0].uri.fsPath, '.agnix.toml');

  const choice = await vscode.window.showQuickPick(
    [
      { label: 'Disable in project', description: 'Add to .agnix.toml', value: 'project' },
      { label: 'Cancel', description: '', value: 'cancel' },
    ],
    {
      title: `Ignore rule ${ruleId}`,
      placeHolder: 'How do you want to ignore this rule?',
    }
  );

  if (!choice || choice.value === 'cancel') {
    return;
  }

  // Read or create .agnix.toml
  let content = '';
  try {
    content = fs.readFileSync(configPath, 'utf-8');
  } catch {
    content = '# agnix configuration\n\n[rules]\ndisabled_rules = []\n';
  }

  // Check if rule already disabled
  if (content.includes(`"${ruleId}"`)) {
    vscode.window.showInformationMessage(`Rule ${ruleId} is already disabled`);
    return;
  }

  // Add rule to disabled_rules
  if (content.includes('disabled_rules = [')) {
    // Add to existing array
    content = content.replace(
      /disabled_rules = \[([^\]]*)\]/,
      (match, rules) => {
        const existingRules = rules.trim();
        if (existingRules === '') {
          return `disabled_rules = ["${ruleId}"]`;
        }
        return `disabled_rules = [${existingRules}, "${ruleId}"]`;
      }
    );
  } else if (content.includes('[rules]')) {
    // Add after [rules] section
    content = content.replace('[rules]', `[rules]\ndisabled_rules = ["${ruleId}"]`);
  } else {
    // Add new [rules] section
    content += `\n[rules]\ndisabled_rules = ["${ruleId}"]\n`;
  }

  fs.writeFileSync(configPath, content);
  vscode.window.showInformationMessage(`Rule ${ruleId} disabled in .agnix.toml`);

  // Trigger revalidation
  if (client) {
    await restartClient();
  }
}

function updateStatusBar(
  state: 'starting' | 'ready' | 'error' | 'disabled',
  text: string
): void {
  statusBarItem.text = `$(file-code) ${text}`;

  switch (state) {
    case 'starting':
      statusBarItem.backgroundColor = undefined;
      statusBarItem.tooltip = 'agnix: Starting language server...';
      break;
    case 'ready':
      statusBarItem.backgroundColor = undefined;
      statusBarItem.tooltip = 'agnix: Ready - Click to show output';
      break;
    case 'error':
      statusBarItem.backgroundColor = new vscode.ThemeColor(
        'statusBarItem.errorBackground'
      );
      statusBarItem.tooltip = 'agnix: Error - Click to show output';
      break;
    case 'disabled':
      statusBarItem.backgroundColor = undefined;
      statusBarItem.tooltip = 'agnix: Disabled';
      break;
  }

  statusBarItem.show();
}

export function deactivate(): Thenable<void> | undefined {
  return lifecycleController?.stop();
}
