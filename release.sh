#/bin/sh
cargo build --release
cp target/release/rvcd ../scaleda/src/main/resources/bin
cargo build --release --target=x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/rvcd.exe ../scaleda/src/main/resources/bin