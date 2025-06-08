# 🦀 backuptool

**backuptool** is a command-line backup tool written in Rust that creates versioned archives of your files using Git-style content hashing and file-level deduplication.

> ⚠️ **Note:** backuptool is currently under active development and has not yet been thoroughly tested. Use it at your own risk and avoid relying on it for critical data — for now!

## ✨ Features

- Create versioned, content-addressed archives
- Compression & Deduplication at the file level using content hashing (similar to Git)
- Backup, Restore specific revisions per logical "channel"
- Verify archive integrity via hash checks
- List available backup channels

## Archive Format
This backup tool organizes data into channels, which act as categories or collections for related files—such as videos on a specific topic. Each channel supports multiple versions, allowing you to back up and restore data snapshots over time. It’s a flexible way to manage and preserve your data in a structured, topic-based format.

## 📦 Installation

Clone and build it yourself:

```bash
git clone https://github.com/awaken1988/backuptool.git
cd backuptool
cargo build --release
```

## Examples

backuptool new

```bash

# Create new Archive
backuptool --archive=/archive_dir new

# Backup the '/mnt/videos' folder into 'media' channel
backuptool --archive=/archive_dir backup --source=/mnt/videos --channel=media

# Restore latest from 'media' channel into temp folder
backuptool --archive=/archive_dir restore --destination=/tmp/videos 

# Verify archive integrity
backuptool --archive=/archive_dir verify

# List all channels
backuptool --archive=/archive_dir list-channel
```


## ✅ TODO
- [ ] Encrypt Files
- [ ] Sync Backup Archive folders among themselves
