<p align="center">
  <img src="images/tzworp-icon.png" width="148" alt="tzWarp">
</p>

<h1 align="center">tzWarp</h1>

<p align="center">
  <b>会说话的中文终端。</b><br>
  日常敲命令，卡住了问一句；命令先给你看，点了才跑。
</p>

<p align="center">
  <a href="README.md">English</a> · <b>简体中文</b>
</p>

<p align="center">
  <a href="https://github.com/kedoupi/tzWarp/releases/latest"><img alt="下载 macOS" src="https://img.shields.io/badge/下载_macOS_Apple_Silicon-v1.0.0-ff6a00?style=for-the-badge"></a>
</p>

<p align="center">
  <img alt="version" src="https://img.shields.io/badge/version-1.0.0-orange">
  <img alt="license" src="https://img.shields.io/badge/license-AGPL--3.0-blue">
  <img alt="platform" src="https://img.shields.io/badge/macOS-12%2B-black">
</p>

<p align="center">
  <a href="#三分钟开始用">三分钟开始用</a> ·
  <a href="#3-注册并填密钥">注册小桃子</a> ·
  <a href="#跟着做一遍">跟着做一遍</a> ·
  <a href="#常见问题">常见问题</a>
</p>

---

tzWarp 是一台真正能天天开着的终端：补全、分屏、块选择、主题都在。上面坐着一个中文智能体，只连你们自己的 **小桃子** 中转，不登录 Warp，也不把对话送去 Warp 云。

适合：不想开 Warp 账号、已经有（或即将有）小桃子密钥、希望「问一句 → 看命令 → 点确认」的人。

## 三分钟开始用

### 1. 下载

到 [Releases](https://github.com/kedoupi/tzWarp/releases/latest) 下载：

- [tzWarp-1.0.0-macos-arm64.dmg](https://github.com/kedoupi/tzWarp/releases/download/v1.0.0/tzWarp-1.0.0-macos-arm64.dmg)（推荐）
- 或 [tzWarp-1.0.0-macos-arm64.zip](https://github.com/kedoupi/tzWarp/releases/download/v1.0.0/tzWarp-1.0.0-macos-arm64.zip)

目前提供 **macOS 12+ / Apple Silicon**。Intel Mac、Windows、Linux 请走[从源码运行](#从源码运行)。

### 2. 安装

1. 打开 DMG，把 **tzWarp** 拖进 **应用程序**
2. 第一次启动：在启动台或 Finder 里 **右键 tzWarp → 打开**  
   （安装包尚未经过 Apple 公证，双击可能被拦截，右键打开一次即可）
3. 系统询问是否打开时选 **打开**

tzWarp 和官方 Warp 可以同时装着，数据互不干扰。

### 3. 注册并填密钥

tzWarp **不用 Warp 账号**。打开 **设置 → 智能体**，就是这个输入框：

<p align="center">
  <img src="images/settings-xiaotaozi-key.png" width="720" alt="设置 → 智能体 → 小桃子 API 密钥">
</p>

按图操作：

1. 打开 [小桃子](https://tzai.kdp.cool) **申请账号**
2. 在控制台复制 API 密钥
3. 粘贴到上图这个 **API 密钥** 框里，回车保存
4. 下面变成绿色「中转服务已连通 · N 个可用模型」就成功了

管理员也可以发激活链接，点开直接写入：

```text
tzworp://activate?token=你的密钥
```

## 跟着做一遍

下面这条路径就是我们自己每天在用的。

**① 当普通终端用 30 秒**  
打开 tzWarp，`cd` 到你的项目，跑 `ls`、`git status`。该补全补全，该分屏分屏。

**② 让智能体看一眼项目**  
在输入框输入（注意是斜杠，不要当 shell 命令敲）：

```text
/plan 这是什么项目呢
```

回车。会进入智能体面板，开始流式回答。不要在系统终端里跑 `/plan`，那是 zsh 路径。

**③ 需要跑命令时：先看卡片，再点确认**  
智能体若要 `ls`、`cat` 某个文件，会弹出确认卡。看清楚命令，点了才执行。执行结果会回到对话里。

**④ 接着问，不用重头讲**  
例如：「做个总结吧」「README 里安装步骤写完整一点」。同一段对话会带着刚才读到的文件继续。

**⑤ 换个模型（可选）**  
智能体顶部的模型选择器来自小桃子的模型列表。选一个你账号能用的即可。

到这里，下载 → 安装 → 注册 → 第一次对话就走通了。

## 你可以用它做什么

| 你想做的 | tzWarp 怎么做 |
|---|---|
| 日常敲命令 | 和 Warp 同级的 GPU 终端 |
| 用中文问项目 | `/plan` 或直接在智能体里提问 |
| 怕 AI 乱执行 | 命令先出卡片，你点了才跑 |
| 多轮往下挖 | 同一对话保留上下文和刚才的输出 |
| 接团队自己的模型 | 只打小桃子中转，不经过 Warp Server |
| 继续用官方 Warp | 可以并存，互不影响 |

## 常见问题

**双击打不开？**  
右键 → 打开。第一次需要在系统对话框里允许。

**智能体说没有密钥 / 拉不到模型？**  
先打开 [小桃子](https://tzai.kdp.cool) 申请账号并复制密钥，再贴回 **设置 → 智能体** 那个输入框。空了就再贴一次。

**`/plan` 变成 `zsh: no such file or directory`？**  
说明命令进了 shell。请在 tzWarp 自己的输入框里输入 `/plan …`，不要在已经跑起来的 zsh 提示符后粘贴。

**智能体复读同一句话？**  
1.0.0 已截住确认执行后的空转。请新开一轮对话再试，不要接着旧的复读记录。

**对话会送到 Warp 吗？**  
不会。请求只到 `https://tzai.kdp.cool/v1`（可用环境变量改中转地址）。

**和官方 Warp 有什么不一样？**  
终端底座来自开源 Warp。没有 Warp 登录、Drive、Cloud Agent、云端交接。智能体是小桃子中转上的对话 + 确认执行，不是官方完整 Oz。

## 从源码运行

```bash
./script/bootstrap --skip-common-skills   # 首次
export TZAI_API_KEY='你的密钥'
./script/run-tzworp
```

自己打安装包：

```bash
./script/package-tzworp
```

接官方 Warp 更新见 [docs/UPSTREAM.md](docs/UPSTREAM.md)。官方原 README：[README.upstream.md](README.upstream.md)。

## 许可证

基于 [Warp](https://github.com/warpdotdev/Warp)，**AGPL-3.0**（UI 框架部分为 MIT）。二次分发须提供对应源码。

tzWarp 不是 Warp 官方产品，未获 Denver Technologies, Inc. 背书。商标归各自所有者。
