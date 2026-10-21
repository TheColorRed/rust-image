Always look for and read one or more skills that are related to the task you are performing.

## Building

- NASM is in the project at `tools/nasm` (`tools/nasm/nasm.exe`). The `rav1d` crate needs it to build, and it is not on the system PATH. Put it on the PATH for the command that builds, for example in Git Bash: `export PATH="$PATH:$PWD/tools/nasm"`.
- Never run `cargo clean` without `-p <crate>`: with no package it deletes the whole `target` folder (tens of GB, and the Android libraries) and everything has to compile again.

## Legacy Code

- When a better way to achieve something exists, replace the old approach with the new one instead of keeping both.
- Prefer faster, more efficient code over legacy approaches, even if that means rewriting working code.
- Prefer approaches that give higher quality image results over legacy approaches.
- Rewrite dependencies and code that rely on a legacy approach to use the new, preferred method, for example by changing the signature and updating every caller rather than leaving a shim for the old one.
