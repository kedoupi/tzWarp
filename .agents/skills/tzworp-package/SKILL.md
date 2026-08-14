---
name: tzworp-package
description: >
  打 tzWarp macOS 正式安装包（release-lto .app / .dmg / .zip），写入版本号并装进 tzWarp.app。
  Use when the user asks to 打包, 安装包, DMG, 1.0.x release binary, or /tzworp-package.
---

# tzworp-package

产品标识见 `docs/TZWARP.md`。不要用 800MB debug 二进制当正式包。

## 做

1. 版本写在仓库根 `VERSION` 和 `app/src/bin/tzworp.rs` 的 `CFBundleShortVersionString` / `CFBundleVersion`。
2. 必须带 `GIT_RELEASE_TAG=v<VERSION>` 编译，关于页才显示版本（`warp_core` 读这个 env）。
3. 在 Warp 源码根执行：

```bash
./script/package-tzworp
# 上层工作区也可用：../scripts/package_macos.sh
```

4. 产物：`dist/tzWarp-<ver>-macos-arm64.dmg` 和 `.zip`（或工作区 `../dist/`）。
5. 重启本地 app 用 `pkill -x tzworp`，**不要** `pkill -f`（会误杀自己）。
6. 未公证：告诉用户第一次右键 → 打开。

## 不要

- 不要 `cargo bundle --channel oss`（那是 WarpOss，不是 tzWarp）。
- 不要打官方 universal / 公证流程（没有 Warp 证书）。
- 不要把 `.app` / `dist/` / `target/` 提交进 git。
