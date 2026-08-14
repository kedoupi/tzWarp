---
name: tzworp-publish
description: >
  把 tzWarp 发布到公开 GitHub：孤儿快照推 github/main，可选挂 Release。
  Use when the user asks to 公开, 推 GitHub, 挂 Release, 刷新快照, or /tzworp-publish.
---

# tzworp-publish

分支职责见 `docs/TZWARP.md` 和 `docs/UPSTREAM.md`。

## 不要

- 不要 `git push github tzworp`。官方历史缺 LFS 对象，会失败。
- 不要往 `github-main` merge `origin/master`。

## 快照

工作区干净、`tzworp` 已提交之后：

```bash
cd Warp   # 若当前就在源码根则省略
TREE=$(git rev-parse 'HEAD^{tree}')
COMMIT=$(git commit-tree "$TREE" -m "feat: tzWarp 快照（tzworp @ $(git rev-parse --short HEAD)）")
git update-ref refs/heads/github-main "$COMMIT"
GIT_LFS_SKIP_PUSH=1 git -c http.postBuffer=524288000 push --no-thin github github-main:main --force
```

若远端还没有这 7 个 LFS 对象，先去掉 `GIT_LFS_SKIP_PUSH=1` 再推一次。

SSH `send-pack` 被掐时：只改文档可用 Contents API（`gh api --method PUT repos/kedoupi/tzWarp/contents/<path>`），不要反复硬推整棵树。

## Release

```bash
gh release create v<VERSION> \
  --repo kedoupi/tzWarp \
  --target "$(git rev-parse github-main)" \
  --title "tzWarp <VERSION>" \
  --notes-file dist/RELEASE-<VERSION>.md \
  dist/tzWarp-<VERSION>-macos-arm64.dmg \
  dist/tzWarp-<VERSION>-macos-arm64.zip
```

公开仓库：`gh repo edit kedoupi/tzWarp --visibility public --accept-visibility-change-consequences`。

## README

- 默认 `README.md` 英文。
- 中文 `README.zh-CN.md`，页头互链。
- 注册流程：截图设置里的密钥框 + 链到 `https://tzai.kdp.cool` 申请，再粘贴。
