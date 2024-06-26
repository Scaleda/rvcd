#/bin/sh
set -ex
if [[ -v BUILD ]]
then
  cargo build --release && \
  cp target/release/rvcd release/ && \
  chmod +x release/rvcd && \
  upx release/rvcd && \
  cargo build --release --target=x86_64-pc-windows-gnu && \
  cp target/x86_64-pc-windows-gnu/release/rvcd.exe release/ && \
  chmod +x release/rvcd.exe && \
  upx release/rvcd.exe && \
  chmod +x release/surfer release/surfer.exe && \
  upx release/surfer release/surfer.exe
fi
# trunk build --release && cp -r dist/ release/
rm -rf release.zip
if [[ -z DELETE_ASSETS ]]
then
  rm ../../scaleda/src/main/resources/bin/assets.zip
fi
# other asserts in release/ will also packed
cd release/ && 7z a ../release.zip -r * && cd ..
cd release/ && 7z a ../../scaleda/scaleda-kernel/src/main/resources/bin/assets.zip -r * && cd ..
