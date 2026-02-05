import * as assert from 'assert';
import * as vscode from 'vscode';
import { buildLspConfig } from '../../extension';

suite('Extension Test Suite', () => {
  vscode.window.showInformationMessage('Start all tests.');

  test('Extension should be present', () => {
    assert.ok(vscode.extensions.getExtension('avifenesh.agnix'));
  });

  test('Extension should activate', async () => {
    const ext = vscode.extensions.getExtension('avifenesh.agnix');
    assert.ok(ext);
    await ext.activate();
    assert.ok(ext.isActive);
  });

  suite('Commands', () => {
    test('agnix.restart command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.restart'));
    });

    test('agnix.showOutput command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.showOutput'));
    });

    test('agnix.validateFile command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.validateFile'));
    });

    test('agnix.validateWorkspace command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.validateWorkspace'));
    });

    test('agnix.showRules command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.showRules'));
    });

    test('agnix.fixAll command should be registered', async () => {
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes('agnix.fixAll'));
    });
  });

  suite('Configuration', () => {
    test('agnix.lspPath should have default value', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const lspPath = config.get<string>('lspPath');
      assert.strictEqual(lspPath, 'agnix-lsp');
    });

    test('agnix.enable should default to true', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const enable = config.get<boolean>('enable');
      assert.strictEqual(enable, true);
    });

    test('agnix.trace.server should default to off', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const trace = config.get<string>('trace.server');
      assert.strictEqual(trace, 'off');
    });

    test('agnix.severity should default to Warning', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const severity = config.get<string>('severity');
      assert.strictEqual(severity, 'Warning');
    });

    test('agnix.target should default to Generic', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const target = config.get<string>('target');
      assert.strictEqual(target, 'Generic');
    });

    test('agnix.tools should default to empty array', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const tools = config.get<string[]>('tools');
      assert.deepStrictEqual(tools, []);
    });
  });

  suite('Rule Configuration', () => {
    const ruleCategories = [
      'skills',
      'hooks',
      'agents',
      'memory',
      'plugins',
      'xml',
      'mcp',
      'imports',
      'crossPlatform',
      'agentsMd',
      'copilot',
      'cursor',
      'promptEngineering',
    ];

    for (const category of ruleCategories) {
      test(`agnix.rules.${category} should default to true`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<boolean>(`rules.${category}`);
        assert.strictEqual(value, true, `rules.${category} should be true by default`);
      });
    }

    test('agnix.rules.disabledRules should default to empty array', () => {
      const config = vscode.workspace.getConfiguration('agnix');
      const disabledRules = config.get<string[]>('rules.disabledRules');
      assert.deepStrictEqual(disabledRules, []);
    });
  });

  suite('Version Configuration', () => {
    const versions = ['claudeCode', 'codex', 'cursor', 'copilot'];

    for (const version of versions) {
      test(`agnix.versions.${version} should default to null`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<string | null>(`versions.${version}`);
        assert.strictEqual(value, null, `versions.${version} should be null by default`);
      });
    }
  });

  suite('Spec Configuration', () => {
    const specs = ['mcpProtocol', 'agentSkills', 'agentsMd'];

    for (const spec of specs) {
      test(`agnix.specs.${spec} should default to null`, () => {
        const config = vscode.workspace.getConfiguration('agnix');
        const value = config.get<string | null>(`specs.${spec}`);
        assert.strictEqual(value, null, `specs.${spec} should be null by default`);
      });
    }
  });

	suite('Configuration Sync', () => {
		test('buildLspConfig should transform camelCase to snake_case', () => {
			const config = buildLspConfig();

			assert.ok(config.rules);
			assert.strictEqual(typeof config.rules.cross_platform, 'boolean');
			assert.strictEqual(typeof config.rules.agents_md, 'boolean');
			assert.strictEqual(typeof config.rules.prompt_engineering, 'boolean');
		});

		test('buildLspConfig should include all configuration sections', () => {
			const config = buildLspConfig();

			assert.ok('severity' in config);
			assert.ok('target' in config);
			assert.ok('tools' in config);
			assert.ok('rules' in config);
			assert.ok('versions' in config);
			assert.ok('specs' in config);
		});

		test('buildLspConfig rules section should have all 14 fields', () => {
			const config = buildLspConfig();

			assert.strictEqual(typeof config.rules?.skills, 'boolean');
			assert.strictEqual(typeof config.rules?.hooks, 'boolean');
			assert.strictEqual(typeof config.rules?.agents, 'boolean');
			assert.strictEqual(typeof config.rules?.memory, 'boolean');
			assert.strictEqual(typeof config.rules?.plugins, 'boolean');
			assert.strictEqual(typeof config.rules?.xml, 'boolean');
			assert.strictEqual(typeof config.rules?.mcp, 'boolean');
			assert.strictEqual(typeof config.rules?.imports, 'boolean');
			assert.strictEqual(typeof config.rules?.cross_platform, 'boolean');
			assert.strictEqual(typeof config.rules?.agents_md, 'boolean');
			assert.strictEqual(typeof config.rules?.copilot, 'boolean');
			assert.strictEqual(typeof config.rules?.cursor, 'boolean');
			assert.strictEqual(typeof config.rules?.prompt_engineering, 'boolean');
			assert.ok(Array.isArray(config.rules?.disabled_rules));
		});
	});

  suite('Language Support', () => {
    test('skill-markdown language should be registered', async () => {
      const languages = await vscode.languages.getLanguages();
      assert.ok(languages.includes('skill-markdown'));
    });
  });
});
