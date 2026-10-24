# 调研与技术参考

查阅日期：2026-10-21。以下为设计参考，产品功能、客户端能力及版本需在实施时复核；不表示下载、安装、兼容或端到端验证已经完成。

## 1. 可参考的现有方案

| 方案 | 已查阅能力 | 对 Tether 的启发 |
| --- | --- | --- |
| [XPipe SSH](https://docs.xpipe.io/guide/ssh) | 借助本机 SSH 客户端和原有配置，组织连接、跳板及隧道 | 最贴近系统 OpenSSH 与本地配置的实现方向 |
| [Bitvise SSH Client](https://bitvise.com/ssh-client) | 图形化转发、自动重连和用户密钥管理 | 参考转发、认证、密钥和运行结果的组织 |
| [MobaXterm 文档](https://mobaxterm.mobatek.net/documentation.html) | 图形化隧道工具、会话及密钥工具 | 参考转发登记和主机组织的操作 |
| [autossh](https://www.harding.motd.ca/autossh/) | 监督 SSH、断线重启、连接监测及失败退避 | 参考重连职责和退避机制，不因此要求依赖 autossh |

选择原生客户端并围绕真实进程与配置做管理，是 Tether 的设计判断。没有必要为首版复制这些产品的完整终端、文件管理或远端运维功能。

## 2. 官方技术依据

| 资料 | 需要核对的内容 |
| --- | --- |
| [OpenSSH ssh 手册](https://man.openbsd.org/ssh) | 转发、多个参数、跳板、配置解析与命令行语义 |
| [OpenSSH 10.0 参数目录来源](https://github.com/openssh/openssh-portable/blob/V_10_0_P1/ssh_config.5) | 参数目录的 102 项配置指令名称；中文标签与表单由 Tether 自行维护，本机支持仍由实际客户端决定 |
| [OpenSSH ssh_config 手册](https://man.openbsd.org/ssh_config) | 配置优先级、Include／Match、保活、转发建立、原生配置能力 |
| [ExitOnForwardFailure](https://man.openbsd.org/ssh_config#ExitOnForwardFailure) | 新组转发建立失败的处理；不保证最终业务目标健康 |
| [ClearAllForwardings](https://man.openbsd.org/ssh_config#ClearAllForwardings) | 会清除文件及命令行来源的转发，不能随意用于只排除文件转发 |
| [ssh-keygen](https://man.openbsd.org/ssh-keygen) | 生成、指纹、公钥及密码短语等能力 |
| [ssh-add](https://man.openbsd.org/ssh-add) | Agent 身份查看、加载、卸载及客户端支持的约束 |
| [Win32 OpenSSH 项目](https://github.com/PowerShell/Win32-OpenSSH) | Windows 客户端版本、发布和平台差异 |
| [Windows ControlMaster 支持问题](https://github.com/PowerShell/Win32-OpenSSH/issues/1328) | 不将跨版本复用控制能力视为默认可用 |
| [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) | 本组进程和子进程归属、组级清理及句柄生命周期 |

OpenBSD 手册描述的能力不保证全部存在于用户本机 Windows 客户端。实际参数、算法和行为必须以所选可执行文件及原生验证为准。

## 3. 适用边界

参考现有交互与官方行为，不复制产品源码或图形资产。Tether 源码采用 [MIT 许可证](../LICENSE)，仓库为 [aisspire/Tether](https://github.com/aisspire/Tether)。第三方依赖保留各自许可证，分发二进制时按实际依赖保留相应声明，不能以项目许可证替代依赖许可证。

Oriel 的已交付网页能力与未来生命周期设计应区分。Tether 文档保存实现所需边界，不依赖父项目路径、被忽略会话或开发机网页作为正式说明。
