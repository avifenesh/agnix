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
    )
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
