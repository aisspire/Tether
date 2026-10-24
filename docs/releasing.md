# 发布说明

源码仓库为 [aisspire/Tether](https://github.com/aisspire/Tether)，项目采用 [MIT 许可证](../LICENSE)。当前版本为 0.1.0 开发预览，Windows 是主要使用平台，Linux x64 静态后台用于既有 WSL 接入。

## 源码公开

本仓库独立维护，可在 Oriel 之外克隆和构建。源码、锁文件、构建入口、测试及正式文档纳入发布；`AGENTS.md` 和设计记录用于说明开发边界，可以保留。真实主机配置、私钥、调用令牌、日志、会话原稿及本机构建目录不进入 Git。

首次推送前检查当前文件与将推送分支的历史。`.gitignore` 不会移除已提交的敏感内容；若有命中，先判断测试样例与真实材料，再处理。只推送明确的分支或标签，避免用 `--mirror` 将本地其他引用一并公开。

在用户明确授权提交与推送后，从本仓库执行：

```bash
git status --short
git remote -v
git push -u origin main
```

执行推送前应先提交已审阅的相关改动；上述命令不会发布未提交文件，也不会自动创建版本标签或 GitHub Release。npm 和 Cargo 包发布保持关闭，GitHub 源码公开不需要发布这些包。

## 二进制发布

从待发布提交在已有 Windows 开发环境中运行 `build.cmd`，沿用项目锁文件。准确构建条件与参数见 [README](../README.md)。运行与改动相关的验证，至少确认最终 Release 包的独立使用与随包资源；检查结果及平台缺口记录在[验证记录](../verification.md)。

交付内容包括 `tether.exe`、`tether-linux-x64`、`dist/`、`plugin.json`、`oriel.ps1`、`USER_GUIDE.md` 和 `LICENSE`。发布二进制前还需按实际依赖整理随包的第三方许可证。不要直接上传整个 `target/` 或本机运行目录，测试数据和 `instance.json` 不属于交付内容。

Windows 打包与最终目录包检查通过后，再制作版本化 ZIP 和 SHA-256 校验值。确认源码版本、标签与产物对应，发布说明列明功能、环境要求和已知限制；开发预览使用 GitHub prerelease 标记。标签、上传附件和公开 Release 均在用户授权后执行。

## 0.1.0 发布文案草稿

Tether 是基于本机 OpenSSH 的 SSH 转发管理工具，可独立使用或接入 Oriel。

- 按主机管理本地、远程和动态 SOCKS 转发，支持固定成员计划、增量启动与连接复用。
- 提供连接状态、认证交互、保活重连、SSH 配置及密钥工具；仅管理自己拥有的运行资源。
- 支持浅深主题、Windows 与既有 WSL 环境、单条映射完整 SSH 命令复制和离线使用说明。

Windows 为主要使用平台，需要已有 OpenSSH。完整验收范围以验证记录为准；PowerShell 7、硬件身份、多因素认证及不同 OpenSSH 版本尚无完整验收矩阵。源码公开不代表二进制附件已发布。
