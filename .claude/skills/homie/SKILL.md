---
name: homie
description: Homie dotfiles symlink orchestrator reference. Use when working with homie commands, homie.toml config files, dotfile repos in ~/.homie/repos/, or managing symlinks with homie.
---

# Homie

Dotfiles symlink orchestrator with templates and multiple repo support.

## Quick reference

```bash
homie link [REPO]           # create symlinks (all or specific repo)
homie unlink [REPO]         # remove symlinks
homie status [REPO]         # show symlink status
homie add <REPO> <FILE>     # move file to repo + symlink
homie diff [REPO]           # show differences
homie init <NAME>           # create new repo
homie clone <URL>           # clone existing repo
homie list                  # list repos
```

Global options: `-n` (dry-run), `-v` (verbose)

## Repo structure

Repos live in `~/.homie/repos/`:

```
~/.homie/repos/dotfiles/
├── homie.toml              # repo config (required)
├── .zshrc                  # links to ~/.zshrc
├── .gitconfig.tmpl         # renders then links to ~/.gitconfig
└── .config/nvim/init.lua   # links to ~/.config/nvim/init.lua
```

## Config: homie.toml

```toml
target = "~"                          # required: symlink target directory

[vars]
email = "me@example.com"              # template variables

[defaults]
strategy = "file"                     # file | directory | contents

[strategies]
".config/nvim" = "directory"          # link entire dir as one symlink
".local/bin" = "contents"             # link each file inside individually

[ignore]
paths = ["*.swp", "scratch/**"]       # additional ignores (globs)

[[imports]]
source = "https://github.com/user/repo.git"
ref = "main"                          # optional: branch/tag/commit
name = "shared"                       # optional: import name
paths = ["*"]                         # optional: paths to include
remap = [{ from = "src", to = "dst" }]  # optional: path remapping
```

## Global config

Optional `~/.config/homie/config.toml`:

```toml
[settings]
backup_suffix = ".backup.%Y%m%d%H%M%S"
replaceable_paths = ["~/generated"]

[vars]
email = "global@example.com"

[env]
pass_through = ["API_KEY"]            # expose as {{env.API_KEY}} in templates
```

## Templates

Files ending in `.tmpl` are rendered with variable substitution:

```
{{var}}           # required (error if missing)
{{var?}}          # optional (empty if missing)
{{var:default}}   # default value
{{env.VAR}}       # environment variable (must be in pass_through)
```

Built-in variables: `{{hostname}}`, `{{user}}`, `{{home}}`, `{{os}}`

## Linking strategies

| Strategy | Behavior |
|----------|----------|
| `file` (default) | Each file gets own symlink |
| `directory` | Entire directory becomes one symlink |
| `contents` | Parent dir created, items inside linked individually |

## Default ignores

Always ignored: `homie.toml`, `.git`, `.homie`, `.DS_Store`, `README.md`, `README`, `LICENSE`, `LICENSE.md`, `.gitignore`

## Status indicators

- `✓` linked correctly
- `~` rendered template
- `→` external symlink (not managed)
- `✗` missing or conflict
