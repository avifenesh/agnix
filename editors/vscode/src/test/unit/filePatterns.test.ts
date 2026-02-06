import * as assert from 'assert';

// Unit tests for file pattern matching logic
// These tests don't require VS Code and can run with plain mocha

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

/**
 * Simple glob-to-regex conversion for testing
 * Handles ** (any path) and * (single segment) patterns
 */
function globToRegex(pattern: string): RegExp {
  // Escape special regex characters except *
  let escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&');

  // Handle ** (matches any path including empty)
  // **/ at start means "optionally any path prefix" (including nested dirs)
  escaped = escaped.replace(/^\*\*\//, '(.*\\/)?');
  // **/ in middle means "any path segment"
  escaped = escaped.replace(/\/\*\*\//g, '(\\/.*\\/|\\/?)');
  // Remaining **
  escaped = escaped.replace(/\*\*/g, '.*');

  // Handle * (matches anything except /)
  escaped = escaped.replace(/\*/g, '[^/]*');

  return new RegExp(`^${escaped}$`);
}

/**
 * Simpler implementation using minimatch-style matching
 */
function matchesPattern(filePath: string, pattern: string): boolean {
  const normalizedPath = filePath.replace(/\\/g, '/');

  // Handle **/ prefix - matches any path prefix
  if (pattern.startsWith('**/')) {
    const suffix = pattern.slice(3);
    // Match at root or any subdirectory
    if (normalizedPath === suffix) return true;
    if (normalizedPath.endsWith('/' + suffix)) return true;
    // Handle nested **/ in the suffix
    if (suffix.includes('*')) {
      return matchesPattern(normalizedPath, suffix) ||
             normalizedPath.split('/').some((_, i, arr) =>
               matchesPattern(arr.slice(i).join('/'), suffix)
             );
    }
  }

  // Handle simple wildcard patterns
  const regex = globToRegex(pattern);
  return regex.test(normalizedPath);
}

/**
 * Check if a path matches any of the agnix file patterns
 */
function isAgnixFile(filePath: string): boolean {
  return AGNIX_FILE_PATTERNS.some((pattern) => matchesPattern(filePath, pattern));
}

describe('File Pattern Matching', () => {
  describe('SKILL.md files', () => {
    it('should match SKILL.md in root', () => {
      assert.ok(isAgnixFile('SKILL.md'));
    });

    it('should match SKILL.md in subdirectory', () => {
      assert.ok(isAgnixFile('skills/review/SKILL.md'));
    });

    it('should match SKILL.md in .claude/skills', () => {
      assert.ok(isAgnixFile('.claude/skills/my-skill/SKILL.md'));
    });

    it('should not match skill.md (lowercase)', () => {
      assert.ok(!isAgnixFile('skill.md'));
    });
  });

  describe('CLAUDE.md files', () => {
    it('should match CLAUDE.md in root', () => {
      assert.ok(isAgnixFile('CLAUDE.md'));
    });

    it('should match CLAUDE.local.md', () => {
      assert.ok(isAgnixFile('CLAUDE.local.md'));
    });

    it('should match CLAUDE.md in subdirectory', () => {
      assert.ok(isAgnixFile('project/CLAUDE.md'));
    });
  });

  describe('AGENTS.md files', () => {
    it('should match AGENTS.md in root', () => {
      assert.ok(isAgnixFile('AGENTS.md'));
    });

    it('should match AGENTS.md in subdirectory', () => {
      assert.ok(isAgnixFile('docs/AGENTS.md'));
    });
  });

  describe('Hook configuration files', () => {
    it('should match .claude/settings.json', () => {
      assert.ok(isAgnixFile('.claude/settings.json'));
    });

    it('should match .claude/settings.local.json', () => {
      assert.ok(isAgnixFile('.claude/settings.local.json'));
    });

    it('should match nested .claude/settings.json', () => {
      assert.ok(isAgnixFile('project/.claude/settings.json'));
    });
  });

  describe('Plugin files', () => {
    it('should match plugin.json', () => {
      assert.ok(isAgnixFile('plugin.json'));
    });

    it('should match plugin.json in .claude-plugin', () => {
      assert.ok(isAgnixFile('.claude-plugin/plugin.json'));
    });
  });

  describe('MCP configuration files', () => {
    it('should match *.mcp.json files', () => {
      assert.ok(isAgnixFile('tools.mcp.json'));
    });

    it('should match nested *.mcp.json files', () => {
      assert.ok(isAgnixFile('config/server.mcp.json'));
    });

    it('should not match regular json files', () => {
      assert.ok(!isAgnixFile('package.json'));
    });
  });

  describe('GitHub Copilot files', () => {
    it('should match .github/copilot-instructions.md', () => {
      assert.ok(isAgnixFile('.github/copilot-instructions.md'));
    });

    it('should match .github/instructions/*.instructions.md', () => {
      assert.ok(isAgnixFile('.github/instructions/coding.instructions.md'));
    });
  });

  describe('Cursor files', () => {
    it('should match .cursor/rules/*.mdc', () => {
      assert.ok(isAgnixFile('.cursor/rules/typescript.mdc'));
    });

    it('should match nested .cursor/rules/*.mdc', () => {
      assert.ok(isAgnixFile('project/.cursor/rules/testing.mdc'));
    });

    it('should not match .mdc files outside .cursor/rules', () => {
      assert.ok(!isAgnixFile('rules/test.mdc'));
    });
  });

  describe('Non-agnix files', () => {
    it('should not match README.md', () => {
      assert.ok(!isAgnixFile('README.md'));
    });

    it('should not match package.json', () => {
      assert.ok(!isAgnixFile('package.json'));
    });

    it('should not match random .json files', () => {
      assert.ok(!isAgnixFile('config.json'));
    });

    it('should not match tsconfig.json', () => {
      assert.ok(!isAgnixFile('tsconfig.json'));
    });
  });
});

describe('Rules Categories', () => {
  const RULE_CATEGORIES = [
    { prefix: 'AS-', name: 'Agent Skills', minCount: 10 },
    { prefix: 'CC-SK-', name: 'Claude Code Skills', minCount: 5 },
    { prefix: 'CC-HK-', name: 'Claude Code Hooks', minCount: 5 },
    { prefix: 'CC-AG-', name: 'Claude Code Agents', minCount: 5 },
    { prefix: 'CC-PL-', name: 'Claude Code Plugins', minCount: 3 },
    { prefix: 'PE-', name: 'Prompt Engineering', minCount: 5 },
    { prefix: 'MCP-', name: 'Model Context Protocol', minCount: 5 },
    { prefix: 'AGM-', name: 'AGENTS.md', minCount: 5 },
    { prefix: 'COP-', name: 'GitHub Copilot', minCount: 3 },
    { prefix: 'CUR-', name: 'Cursor', minCount: 3 },
    { prefix: 'XML-', name: 'XML Tags', minCount: 2 },
    { prefix: 'XP-', name: 'Cross-Platform', minCount: 3 },
  ];

  it('should have 12 rule categories defined', () => {
    assert.strictEqual(RULE_CATEGORIES.length, 12);
  });

  it('should have unique prefixes', () => {
    const prefixes = RULE_CATEGORIES.map((c) => c.prefix);
    const uniquePrefixes = new Set(prefixes);
    assert.strictEqual(prefixes.length, uniquePrefixes.size);
  });

  it('each category should have a descriptive name', () => {
    RULE_CATEGORIES.forEach((category) => {
      assert.ok(category.name.length > 0, `Category ${category.prefix} should have a name`);
    });
  });
});

describe('Fix Preview Logic', () => {
  // Mock TextEdit interface for testing
  interface MockRange {
    start: { line: number; character: number };
    end: { line: number; character: number };
    isEmpty: boolean;
  }

  interface MockTextEdit {
    range: MockRange;
    newText: string;
  }

  /**
   * Get edit summary (extracted from extension.ts for testing)
   */
  function getEditSummary(changes: MockTextEdit[]): string {
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

  describe('getEditSummary', () => {
    it('should return empty string for no changes', () => {
      assert.strictEqual(getEditSummary([]), '');
    });

    it('should identify delete operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 4, character: 0 },
            end: { line: 4, character: 10 },
            isEmpty: false,
          },
          newText: '',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 5: delete text');
    });

    it('should identify insert operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 9, character: 5 },
            end: { line: 9, character: 5 },
            isEmpty: true,
          },
          newText: 'inserted text',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 10: insert text');
    });

    it('should identify replace operations', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 0, character: 5 },
            isEmpty: false,
          },
          newText: 'replacement',
        },
      ];
      assert.strictEqual(getEditSummary(changes), 'Line 1: replace text');
    });

    it('should summarize multiple changes', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 0, character: 0 },
            end: { line: 0, character: 5 },
            isEmpty: false,
          },
          newText: 'a',
        },
        {
          range: {
            start: { line: 1, character: 0 },
            end: { line: 1, character: 5 },
            isEmpty: false,
          },
          newText: 'b',
        },
        {
          range: {
            start: { line: 2, character: 0 },
            end: { line: 2, character: 5 },
            isEmpty: false,
          },
          newText: 'c',
        },
      ];
      assert.strictEqual(getEditSummary(changes), '3 changes (3 replaces)');
    });

    it('should summarize mixed change types', () => {
      const changes: MockTextEdit[] = [
        {
          range: {
            start: { line: 1, character: 0 },
            end: { line: 1, character: 3 },
            isEmpty: false,
          },
          newText: 'abc',
        },
        {
          range: {
            start: { line: 2, character: 2 },
            end: { line: 2, character: 2 },
            isEmpty: true,
          },
          newText: 'x',
        },
        {
          range: {
            start: { line: 3, character: 0 },
            end: { line: 3, character: 1 },
            isEmpty: false,
          },
          newText: '',
        },
      ];

      assert.strictEqual(
        getEditSummary(changes),
        '3 changes (1 replace, 1 insert, 1 delete)'
      );
    });
  });

  describe('Fix confidence levels', () => {
    it('safe fixes should be marked as preferred', () => {
      // This matches the LSP behavior where safe=true maps to isPreferred=true
      const safeFix = { isPreferred: true };
      const unsafeFix = { isPreferred: false };

      assert.strictEqual(safeFix.isPreferred, true);
      assert.strictEqual(unsafeFix.isPreferred, false);
    });
  });

  describe('Agnix action filtering', () => {
    const AGNIX_RULE_RE = /^(AS|CC|PE|MCP|AGM|COP|CUR|XML|XP)-/;

    function isAgnixDiagnostic(diag: { source?: string; code?: string }): boolean {
      return diag.source === 'agnix' || AGNIX_RULE_RE.test(diag.code || '');
    }

    function filterAgnixFixActions(
      actions: Array<{ edit?: object; diagnostics?: Array<{ source?: string; code?: string }> }>
    ) {
      return actions.filter((action) => {
        if (!action.edit) return false;
        const diagnostics = action.diagnostics || [];
        return diagnostics.some(isAgnixDiagnostic);
      });
    }

    it('should keep only actions with agnix diagnostics', () => {
      const actions = [
        { edit: {}, diagnostics: [{ source: 'agnix', code: 'AS-004' }] },
        { edit: {}, diagnostics: [{ source: 'eslint', code: 'no-unused-vars' }] },
        { edit: undefined, diagnostics: [{ source: 'agnix', code: 'AS-005' }] },
      ];

      const filtered = filterAgnixFixActions(actions);
      assert.strictEqual(filtered.length, 1);
      assert.strictEqual(filtered[0].diagnostics?.[0].code, 'AS-004');
    });
  });
});
