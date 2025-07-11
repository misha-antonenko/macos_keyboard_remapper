# `macos_keyboard_remapper`: remap Dvorak to QWERTY on macOS

[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/macos_keyboard_remapper.svg
[crates-url]: https://crates.io/crates/macos_keyboard_remapper

this simple macOS daemon will remap keys from the "Dvorak — QWERTY Cmd" layout to US QWERTY layout _iff_
the "control" or "function" keys are pressed ("command" is already covered by "Dvorak — QWERTY Cmd").
this is done in order to preserve compatibility with apps that define their keybindings relative to
QWERTY. so, you are free to use any QWERTY keybindings that employ "control", "function", or "command",
and if a keybinding uses "alt", you can typically just press "function", because it most often is
a no-op in keybindings

## why another keyboard remapper?

save for Karabiner-Elements, i have not found any other tools that have similar functionality.
but the latter forces me to learn a complicated JSON-based DSL to perform the same simple task.
i could not figure out how to make it respect "caps lock" with Dvorak, or how to still remap to
QWERTY while an "alt" is pressed

# installation

1. install the binary into your `~/.cargo/bin` with `cargo install macos_keyboard_remapper`
2. make sure that `~/.cargo/bin` is in `$PATH`
3. run `macos_keyboard_remapper install` to install the service. you will be asked for permission to
  control your computer; grant it
4. wait a few seconds and enjoy

# deinstallation

1. run `macos_keyboard_remapper uninstall`
2. remove the granted accessibility permission in system settings

# acknowledgements

most of the work was done by o4-mini and [OpenAI Codex](https://github.com/openai/codex), huge
thanks to the team that made them