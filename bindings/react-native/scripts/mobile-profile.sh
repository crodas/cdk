# Size settings for the mobile artifacts, applied as Cargo env overrides rather
# than a named profile: ubrn passes `--profile release` itself and looks for
# the output under `target/<triple>/release`, so a second `--profile` both
# conflicts on the command line and moves the artifact out from under it.
#
# Deliberately not panic=abort, which the workspace's `release-smaller` profile
# sets. UniFFI's scaffolding wraps every call in catch_unwind to turn a Rust
# panic into an error the caller can handle; aborting would take the whole app
# down instead.
# No LTO. iOS ships a static archive, so LTO only makes rustc emit LLVM
# bitcode instead of object code and defers the optimisation to the app's
# link step: measured here, fat LTO took the xcframework from 143 MB to
# 283 MB. The app's linker dead-strips at link time either way.
export CARGO_PROFILE_RELEASE_OPT_LEVEL=z
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=debuginfo
