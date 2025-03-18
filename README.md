# runi

![image](https://user-images.githubusercontent.com/14338722/192661459-325ab1b4-58f5-430a-a857-7aa5f61862fa.png)


## Usage

Just bind `runi` to your favorite keyboard shortcut and select an app to launch.

## Features

* scans common paths for `.desktop` files
* allows rewriting `Exec` value

You can define exec overrides in `$HOME/.config/runi/runi.toml`

### Example

We want to launch some Electron app in native Wayland mode:

```shell
# `~/.config/runi/runi.toml`

[patch."/usr/share/applications/my-electron-app.desktop"]
exec = "my-electron-app --enable-features=UseOzonePlatform --ozone-platform=wayland -- %u"
```


## Installation

Requirements:

* Rust (tested on 1.85 stable)

```shell
cargo install --git https://github.com/mpajkowski/runi.git
```
