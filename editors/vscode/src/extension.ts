import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

/**
 * File patterns that agnix validates.
 */
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

  // Create status bar item
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.command = 'agnix.showOutput';
  context.subscriptions.push(statusBarItem);

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('agnix.restart', () => restartClient()),
    vscode.commands.registerCommand('agnix.showOutput', () =>
      outputChannel.show()
    )
  );

  // Watch for configuration changes
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration('agnix')) {
        const config = vscode.workspace.getConfiguration('agnix');
        if (!config.get<boolean>('enable', true)) {
          stopClient();
        } else {
          restartClient();
        }
      }
    })
  );

  // Start the client if enabled
  const config = vscode.workspace.getConfiguration('agnix');
  if (config.get<boolean>('enable', true)) {
    await startClient();
  }
}

async function startClient(): Promise<void> {
  const config = vscode.workspace.getConfiguration('agnix');
  const lspPath = config.get<string>('lspPath', 'agnix-lsp');

  // Check if the LSP binary exists
  const lspExists = await checkLspExists(lspPath);
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

async function checkLspExists(lspPath: string): Promise<boolean> {
  const { exec } = require('child_process');
  const { promisify } = require('util');
  const execAsync = promisify(exec);

  // Handle Windows where .exe extension might be needed
  const isWindows = process.platform === 'win32';
  const command = isWindows ? `where ${lspPath}` : `which ${lspPath}`;

  try {
    await execAsync(command);
    return true;
  } catch {
    // If not in PATH, check if it's an absolute path that exists
    const fs = require('fs');
    try {
      fs.accessSync(lspPath, fs.constants.X_OK);
      return true;
    } catch {
      return false;
    }
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
