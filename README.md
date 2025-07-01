# ftdv - File Tree Diff Viewer

ftdv (File Tree Diff Viewer) is a terminal-based diff viewer inspired by [diffnav](https://github.com/dlvhdr/diffnav) and [lazygit](https://github.com/jesseduffield/lazygit), built with [ratatui](https://github.com/ratatui-org/ratatui) in Rust. It combines diffnav's intuitive file navigation with lazygit's flexible diff tool configuration system, providing an interactive interface for viewing git diffs with support for various diff tools like delta, bat, ydiff, and difftastic.

## about
<img width="2056" alt="image" src="https://github.com/user-attachments/assets/43ab0c27-0764-4342-a049-fb4930463811" />

## Design Philosophy

ftdv is a modern TUI diff pager focused on interactive git integration:
- **Direct git integration**: Native support for git operations (`ftdv branch1 branch2`)
- **File-by-file diffs**: Fresh diff content for each selected file
- **External tool integration**: Flexible configuration for delta, difftastic, bat, ydiff, etc.
- **Interactive navigation**: File tree interface with persistent state

## Features

- Interactive file tree navigation (inspired by diffnav) with directory folding
- Flexible diff tool configuration with template variables (inspired by lazygit)
- Support for multiple diff tools: delta, bat, ydiff, difftastic, and more
- ANSI color support - automatically detects and renders colored output
- Checkbox functionality to mark reviewed files (review tracking)
- Persistent state management for checked files
- Search functionality with real-time filtering
- Vim-style keyboard navigation
- Customizable themes and colors
- Direct file/directory comparison support
- Git integration with multiple operation modes

## Installation

```bash
# Install from crates.io
cargo install ftdv

# Or clone and build from source
git clone https://github.com/wtnqk/ftdv.git
cd ftdv
cargo install --path .
```

## Usage

### Basic Commands

```bash
# View working directory changes (default)
ftdv

# View staged changes
ftdv --cached

# Compare with a specific commit/branch
ftdv main

# Compare two commits/branches
ftdv main feature-branch

# Compare two files
ftdv file1.txt file2.txt

# Compare two directories
ftdv dir1/ dir2/

# Generate shell completions
ftdv completions bash > ftdv.bash
```

### Keyboard Shortcuts

#### Navigation
| Key      | Action                        |
|----------|-------------------------------|
| `j` / `↓`| Move down in file list        |
| `k` / `↑`| Move up in file list          |
| `g`      | Jump to top of file list      |
| `G`      | Jump to bottom of file list   |

#### Diff Content Scrolling
| Key              | Action                      |
|------------------|-----------------------------|
| `h` / `←`        | Scroll diff left (5 chars)  |
| `l` / `→`        | Scroll diff right (5 chars) |
| `H`              | Scroll diff left (20 chars) |
| `L`              | Scroll diff right (20 chars)|
| `e` / `J`        | Scroll diff down (1 line)   |
| `y` / `K`        | Scroll diff up (1 line)     |
| `d` / `PageDown` | Scroll diff down (10 lines) |
| `u` / `PageUp`   | Scroll diff up (10 lines)   |
| `f`              | Scroll diff down (20 lines) |
| `b`              | Scroll diff up (20 lines)   |

#### File Operations
| Key     | Action                      |
|---------|-----------------------------|
| `Enter` | Expand/collapse directory   |
| `Space` | Update diff content         |
| `Tab`   | Toggle file checkbox        |

#### Search
| Key        | Action                      |
|------------|-----------------------------|
| `/`        | Enter search mode           |
| `Enter`    | Confirm search (in search)  |
| `Esc`      | Exit search mode            |
| `Backspace`| Remove character (in search)|

#### Application
| Key     | Action                      |
|---------|-----------------------------|
| `q`     | Quit application            |
| `Esc`   | Quit application (if not in search)|

## Configuration

ftdv uses a YAML configuration file located at `~/.config/ftdv/config.yaml`.

### Basic Configuration Structure

```yaml
# Git paging configuration
git:
  paging:
    # For stdin/stdout based tools (delta, bat, ydiff)
    pager: "command with options"
    
    # For external diff tools (difftastic)
    externalDiffCommand: "command with options"
    
    # Color argument passed to git
    colorArg: "always"
    
    # Use git's configured pager
    useConfig: false

# Theme configuration
theme:
  name: dark
  colors:
    # Color definitions...
```

### Diff Tool Configuration

#### Delta (Recommended)
```yaml
git:
  paging:
    pager: "delta --dark --paging=never --line-numbers --side-by-side -w={{diffAreaWidth}}"
    colorArg: "always"
```

#### bat
```yaml
git:
  paging:
    pager: "bat --style=plain --color=always --terminal-width={{diffAreaWidth}}"
    colorArg: "always"
```

#### ydiff
```yaml
git:
  paging:
    pager: "ydiff -p cat --color=always --theme=dark --width={{diffAreaWidth}}"
    colorArg: "always"
```

#### difftastic
```yaml
git:
  paging:
    # Note: difftastic uses externalDiffCommand, not pager
    externalDiffCommand: "difft --color=always --background dark --width {{diffAreaWidth}}"
    colorArg: "always"
```

### Template Variables

The following template variables (inspired by lazygit's configuration system) can be used in pager and externalDiffCommand strings:

| Variable             | Description                              |
|----------------------|------------------------------------------|
| `{{width}}`          | Full terminal width                      |
| `{{columnWidth}}`    | Half terminal width minus padding        |
| `{{diffAreaWidth}}`  | Width of the diff display area (80%)     |
| `{{diffColumnWidth}}`| Half of diff area width for side-by-side|

### Theme Configuration

#### Color Options

Colors can be specified using:
- Named colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`
- Gray variants: `gray`, `dark_gray`
- Light variants: `light_red`, `light_green`, `light_yellow`, etc.
- RGB hex codes: `#ff0000`, `#00ff00`, `#323264`
- 256 color palette: `color0` through `color255`

#### Example Themes

##### Dark Theme (Default)
```yaml
theme:
  name: dark
  colors:
    # File tree
    tree_line: dark_gray
    tree_selected_bg: "#323264"
    tree_selected_fg: yellow
    tree_directory: blue
    tree_file: white
    
    # Status
    status_added: green
    status_removed: red
    status_modified: yellow
    
    # UI
    border: dark_gray
    border_focused: cyan
    title: cyan
    status_bar_bg: dark_gray
    status_bar_fg: white
    
    # Text
    text_primary: white
    text_secondary: gray
    text_dim: dark_gray
    
    # Background
    background: black
```

##### Light Theme
```yaml
theme:
  name: light
  colors:
    tree_selected_bg: "#e6e6fa"
    tree_selected_fg: black
    tree_directory: blue
    tree_file: black
    status_added: green
    status_removed: red
    border: gray
    border_focused: blue
    text_primary: black
    background: white
```

## Advanced Usage

### Working with Different Diff Tools

#### Pager vs External Diff Command

- **Pager**: Tools that accept diff content via stdin (delta, bat, ydiff)
- **External Diff Command**: Tools that work with Git's external diff mechanism (difftastic)

The distinction is important because:
1. Pagers receive the diff content through stdin
2. External diff tools are invoked by Git with file paths as arguments

### Custom Diff Tool Integration

To add a new diff tool:

1. Determine if it's a pager or external diff tool
2. Add the appropriate configuration:

```yaml
# For a pager tool
git:
  paging:
    pager: "your-tool --option1 --width={{diffAreaWidth}}"
    colorArg: "always"

# For an external diff tool
git:
  paging:
    externalDiffCommand: "your-tool --option1 --width {{diffAreaWidth}}"
    colorArg: "always"
```

### Persistence

ftdv stores persistent data in `~/.local/share/ftdv/`:
- Checked file states are preserved between sessions

## Troubleshooting

### Common Issues

#### Diff tool not working
1. Ensure the tool is installed and in your PATH
2. Check if it should use `pager` or `externalDiffCommand`
3. Verify the command syntax in your config

#### Width issues
- Use `{{diffAreaWidth}}` for most cases
- Some tools may need `{{width}}` for full terminal width
- Check if the tool reads `COLUMNS` environment variable

#### Colors not showing
- Ensure `colorArg: "always"` is set
- Some tools may need additional color flags

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
