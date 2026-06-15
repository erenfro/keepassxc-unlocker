# KeePassXC Unlocker (Rust)

A portable, efficient background utility that automatically unlocks KeePassXC databases based on D-Bus signals and process events.

**Current Version**: 1.1.0

## Features
- **Auto-Unlock on Login/Unlock**: Listens for screensaver interfaces to unlock your databases when you unlock/lock your session.
- **Process Monitoring**: Automatically detects when the KeePassXC process starts and triggers an unlock request.
- **Session Awareness**: Intelligently skips unlock attempts while the screen is locked to avoid race conditions.
- **Periodic Re-Unlock**: Configurable auto-unlock interval that verifies that your databases are open (while the session is active), ensuring your vault stays ready even if manually locked or closed.
- **Secure Storage**: Uses the system SecretService keyring to store database passwords.
- **Single Binary**: Compiles to a single, portable binary with no runtime dependencies like Python.
- **Systemd Integration**: Built-in commands to install and manage a user-level background service.
- **Shell Auto-Completion**: Automatically generates shell completion scripts for bash/zsh.

## Installation & Setup

### 1. Build from Source
Ensure you have the Rust toolchain installed.
```bash
cargo build --release
# The binary will be at target/release/keepassxc-unlocker
```

### 2. Configure Databases
Add your KeePassXC database(s) to the unlocker. This will prompt for your database password and store it securely in your system keyring.
```bash
keepassxc-unlocker add /path/to/your/database.kdbx
```

### 3. Install as a Service
Enable the utility to run automatically in the background as a systemd user service.
```bash
keepassxc-unlocker service add
```

## CLI Usage

```text
Usage: keepassxc-unlocker [OPTIONS] <COMMAND>

Commands:
  add <DB_PATH>     Add an entry to the keyring and enable it in config
  remove <DB_PATH>  Remove an entry from the keyring and config
  list              List all configured databases and their status
  completion <SH>   Generate shell completion scripts (bash, zsh, fish, etc.)
  service <ACTION>  Manage the systemd user service (add, remove, status)
  unlock            Manually trigger an unlock of all enabled databases
  watch             Start the background monitor (used by the service)
  version           Show version information
  help              Print help information

Options:
  -v, --version     Show version information
  -h, --help        Print help
```

## Shell Completions
To enable auto-completion for your shell, add the following to your configuration:

### Bash
```bash
source <(keepassxc-unlocker completion bash)
```

### Zsh
```zsh
source <(keepassxc-unlocker completion zsh)
```

## Configuration
The tool maintains compatibility with the original configuration file at:
`~/.config/keepassxc-unlockerrc`

### Configuration Options

#### `[monitor]` section:
- `process`: The name of the KeePassXC process to monitor (default: `keepassxc`).
- `service`: The name of the keyring service (default: `keepassxc-unlocker`).
- `autounlock`: Periodic auto-unlock interval in seconds. Set to `0` to disable (default: `0`).

You can manually enable/disable databases in the `[databases]` section.

## Requirements
- **KeePassXC**: Must have "Allow Browser Integration" or "Enable D-Bus" options enabled for the `openDatabase` call to work.
- **Linux**: Requires a D-Bus session bus and a compatible screensaver interface (GNOME, KDE, XFCE).
- **Keyring**: A running secret service (like `gnome-keyring` or `kwallet`).

## License
GNU General Public License v3.0 (GPLv3)
