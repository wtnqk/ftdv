# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-07-01

### Added
- Initial release of ftdv (File Tree Diff Viewer)
- Interactive file tree navigation with directory folding
- Support for multiple diff tools (delta, bat, ydiff, difftastic)
- Template variable system for flexible diff tool configuration
- Search functionality with real-time filtering
- Persistent checkbox state for reviewed files
- Vim-style keyboard navigation
- Customizable themes and colors
- Direct file/directory comparison support
- Git integration with multiple operation modes
- Shell completion support (bash, zsh, fish, etc.)

### Features
- Native git integration without requiring stdin piping
- Lazygit-style configuration system
- Cross-platform support (Linux, macOS, Windows)
- ANSI color support with automatic detection
- Efficient diff rendering with scrolling support

[Unreleased]: https://github.com/yourusername/ftdv/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yourusername/ftdv/releases/tag/v0.1.0