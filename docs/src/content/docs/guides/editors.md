---
title: Editor Integration
description: Integrate FSH Lint with your code editor
---

Enable FSH Lint in your favorite editor for real-time linting feedback.

## VS Code

### FSH Language Extension

Install the FSH extension:

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "FSH Language Support"
4. Install the extension

### Configure FSH Lint

Add to `.vscode/settings.json`:

```json
{
  "fsh.lint.enabled": true,
  "fsh.lint.run": "onType",
  "fsh.lint.path": "fsh-lint",
  "fsh.lint.autoFixOnSave": true
}
```

### Tasks

Create `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "FSH Lint",
      "type": "shell",
      "command": "fsh-lint",
      "args": ["lint", "${workspaceFolder}/**/*.fsh"],
      "problemMatcher": []
    }
  ]
}
```

## JetBrains IDEs

### File Watcher

1. Go to Settings → Tools → File Watchers
2. Click "+" to add new watcher
3. Configure:
   - Name: FSH Lint
   - File type: FSH (or Custom)
   - Scope: Project Files
   - Program: `fsh-lint`
   - Arguments: `lint --fix $FilePath$`
   - Working directory: `$ProjectFileDir$`

## Vim/Neovim

### ALE Integration

Add to `.vimrc` or `init.vim`:

```vim
let g:ale_linters = {
\   'fsh': ['fsh-lint'],
\}

let g:ale_fixers = {
\   'fsh': ['fsh-lint'],
\}

let g:ale_fix_on_save = 1
```

### Configure LSP

Coming soon - LSP server for FSH Lint.

## Emacs

### Flycheck

Add to your Emacs config:

```elisp
(require 'flycheck)

(flycheck-define-checker fsh-lint
  "FSH linter using fsh-lint."
  :command ("fsh-lint" "lint" source)
  :error-patterns
  ((error line-start (file-name) ":" line ":" column ": error: " (message))
   (warning line-start (file-name) ":" line ":" column ": warning: " (message)))
  :modes (fsh-mode))

(add-to-list 'flycheck-checkers 'fsh-lint)
```

## Sublime Text

### SublimeLinter Integration

1. Install SublimeLinter package
2. Create plugin at `Packages/User/linter_fsh.py`:

```python
from SublimeLinter.lint import Linter

class FshLint(Linter):
    cmd = 'fsh-lint lint ${file}'
    regex = r'^.+:(?P<line>\d+):(?P<col>\d+):\s+(?:(?P<error>error)|(?P<warning>warning)):\s+(?P<message>.+)$'
    tempfile_suffix = 'fsh'
```

## Generic Editor Setup

For editors without specific plugins:

### Lint on Save

Configure your editor to run on save:

```bash
fsh-lint lint --fix /path/to/file.fsh
```

### External Tool

Set up FSH Lint as an external tool with:
- Command: `fsh-lint`
- Arguments: `lint --fix $FILE`
- Working directory: `$PROJECT_DIR`

## Features by Editor

| Feature | VS Code | JetBrains | Vim | Emacs | Sublime |
|---------|---------|-----------|-----|-------|---------|
| Syntax Highlighting | ✓ | ✓ | ✓ | ✓ | ✓ |
| Real-time Linting | ✓ | ✓ | ✓ | ✓ | ✓ |
| Auto-fix on Save | ✓ | ✓ | ✓ | ✓ | - |
| Quick Fixes | ✓ | - | - | - | - |
| Rule Documentation | ✓ | - | - | - | - |

## Troubleshooting

### Editor Can't Find FSH Lint

Add to your editor's PATH or specify full path:

```json
{
  "fsh.lint.path": "/usr/local/bin/fsh-lint"
}
```

### Linting Too Slow

Adjust lint frequency:

```json
{
  "fsh.lint.run": "onSave"  // Instead of "onType"
}
```
