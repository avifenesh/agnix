import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

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

export async function activate(
  context: vscode.ExtensionContext
): Promise<void> {
  outputChannel = vscode.window.createOutputChannel('agnix');
  context.subscriptions.push(outputChannel);

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.command = 'agnix.showOutput';
  context.subscriptions.push(statusBarItem);

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
    vscode.commands.registerCommand('agnix.fixAll', () => fixAllInFile())
  );

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration('agnix')) {
        const config = vscode.workspace.getConfiguration('agnix');
        if (!config.get<boolean>('enable', true)) {
          await stopClient();
        } else {
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
  const config = vscode.workspace.getConfiguration('agnix');
  const lspPath = config.get<string>('lspPath', 'agnix-lsp');

  const lspExists = checkLspExists(lspPath);
  if (!lspExists) {
    updateStatusBar('error', 'agnix-lsp not found');
    outputChannel.appendLine(`Error: Could not find agnix-lsp at: ${lspPath}`);
    outputChannel.appendLine('');
    outputChannel.appendLine('To install agnix-lsp:');
    outputChannel.appendLine('  cargo install --path crates/agnix-lsp');
    outputChannel.appendLine('');
    outputChannel.appendLine('Or set the path in settings:');
    outputChannel.appendLine('  "agnix.lspPath": "/path/to/agnix-lsp"');

    vscode.window
      .showErrorMessage(
        'agnix-lsp not found. Install it with: cargo install --path crates/agnix-lsp',
        'Open Settings'
      )
      .then((selection) => {
        if (selection === 'Open Settings') {
          vscode.commands.executeCommand(
            'workbench.action.openSettings',
            'agnix.lspPath'
          );
        }
      });
    return;
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

  client = new LanguageClient(
    'agnix',
    'agnix Language Server',
    serverOptions,
    clientOptions
  );

  try {
    await client.start();
    outputChannel.appendLine('agnix-lsp started successfully');
    updateStatusBar('ready', 'agnix');
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Failed to start agnix-lsp: ${message}`);
    updateStatusBar('error', 'agnix (error)');
    vscode.window.showErrorMessage(`Failed to start agnix-lsp: ${message}`);
  }
}

async function stopClient(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
  updateStatusBar('disabled', 'agnix (disabled)');
}

async function restartClient(): Promise<void> {
  outputChannel.appendLine('Restarting agnix-lsp...');
  if (client) {
    await client.stop();
    client = undefined;
  }
  await startClient();
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
  const agnixDiagnostics = diagnostics.filter(
    (d) => d.source === 'agnix' || d.code?.toString().match(/^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/)
  );

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

  if (!actions || actions.length === 0) {
    vscode.window.showInformationMessage(
      'No automatic fixes available for current issues'
    );
    return;
  }

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
    vscode.window.showInformationMessage(
      'No automatic fixes could be applied'
    );
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
  if (!client) {
    return undefined;
  }
  return client.stop();
}
