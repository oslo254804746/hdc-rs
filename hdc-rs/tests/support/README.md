# OpenHarmony TCP stdio bridge

`tcp_stdio_bridge.c` is the device-side helper used by the ignored fport and
rport data-plane tests. It is intentionally a small single-connection IPv4
program: `-l -p PORT` listens once on device loopback, while `127.0.0.1 PORT`
connects once. The poll loop transfers bytes in both directions between
standard input/output and the socket. It does not implement TLS or any HDC
protocol framing.

The checked-in source can be rebuilt with the OpenHarmony Native SDK already
installed on the validation workstation. Run these commands from the
repository root in PowerShell:

```powershell
$sdkNative = 'C:\Users\wangbaofeng\bin\command-line-tools\sdk\default\openharmony\native'
$clang = Join-Path $sdkNative 'llvm\bin\clang.exe'
$sysroot = Join-Path $sdkNative 'sysroot'
$output = Join-Path (Get-Location) 'target\revalidation-20260904\helper'
New-Item -ItemType Directory -Force -Path $output | Out-Null
& $clang `
  --target=aarch64-linux-ohos `
  --sysroot=$sysroot `
  -D__MUSL__ `
  -std=c11 -O2 -pipe -Wall -Wextra -Werror `
  -ffunction-sections -fdata-sections `
  -fno-ident -static '-Wl,--gc-sections' '-Wl,--as-needed' -s `
  (Join-Path (Get-Location) 'hdc-rs\tests\support\tcp_stdio_bridge.c') `
  -o (Join-Path $output 'tcp_stdio_bridge')
if ($LASTEXITCODE -ne 0) { throw "OpenHarmony bridge build failed ($LASTEXITCODE)" }

& (Join-Path $sdkNative 'llvm\bin\llvm-readelf.exe') -h -l (Join-Path $output 'tcp_stdio_bridge')
Get-FileHash (Join-Path $output 'tcp_stdio_bridge') -Algorithm SHA256
```

The static ELF has no `PT_INTERP` segment, so it is independent of device-side
shared-library search paths; the SDK sysroot supplies the OpenHarmony musl
startup objects and libc. (The device's dynamic musl interpreter, if needed by
another build, is `/lib/ld-musl-aarch64.so.1`.) The resulting path is
`target/revalidation-20260904/helper/tcp_stdio_bridge`.

For the real-device tests, upload this exact file only after recording the
local SHA-256 and agreeing on a device-side path, then set
`HDC_TEST_NC` to that path. The test's existing FIFO pipeline supplies its
initial payload and reads the peer payload back through the bridge.
