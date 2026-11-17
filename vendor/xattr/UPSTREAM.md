# xattr 0.2.3 compatibility patch

Source: https://crates.io/crates/xattr/0.2.3 and https://github.com/Stebalien/xattr.
Upstream source and its MIT/Apache-2.0 licenses are retained here.

GPUI 0.2.2 depends on zed-async-tar 0.5.0-zed, which still requires xattr 0.2.
Recent libc no longer exports the deprecated Linux/Android ENOATTR alias. This
backport changes only src/sys/mod.rs: use ENODATA on Linux/Android; retain ENOATTR
on platforms where it is actually defined. No filesystem semantics or APIs are
changed. Upstream tests are preserved.

Remove this Cargo patch when GPUI's dependency chain uses a maintained xattr.
This dependency is not governed by the application's Commons Clause.
