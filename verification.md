# 实际验证与剩余工作

日期：2026-10-21。此记录区分已实现、实际通过与仍需完善的能力；不把设计验收矩阵整体标为通过。

## 本轮命令与结果

| 仓库 / 环境 | 实际检查 | 结果 |
| --- | --- | --- |
| Tether / WSL Rust 1.98 | `cargo test --locked` | 5 项通过：配置损坏与外部写入保护、托管选项约束、IPv6 参数、继承规则和冲突模型 |
| Tether / WSL | `cargo clippy --locked --all-targets -- -D warnings` | 通过 |
| Tether / Windows Node | `npm run build` | Vue / TypeScript 检查与 Vite 生产构建通过 |
| Tether / Windows Rust + 系统 OpenSSH | `cargo build --locked`、`TETHER_USE_WRAPPER=1 node scripts/check-runtime.mjs` | 20 项原生隔离场景通过，包含实际 PowerShell 生命周期入口 |
| Tether / Windows Edge | `node scripts/check-ui.mjs` | 4 项通过：主机、规则、计划真实保存；浅深主题、40 条长列表、480px 窄屏与焦点；已审查截图 |
| Tether / Windows | `build.ps1`、`node scripts/check-bundle.mjs` | Release 目录包生成；仓库外工作目录资源、SSH check、CLI 状态、stdin 保持打开时退出 4 项冒烟通过；未安装或发布 |
| Oriel / WSL | `cargo test --manifest-path src-tauri/Cargo.toml -p oriel-core --locked` | 22 项核心测试通过，含显式服务能力校验与网页编辑保留依赖 |
| Oriel / Windows | 前端生产构建、Windows Debug 宿主构建、`node scripts/check-tether.mjs`、`npm run check:browser` | 构建通过，8 项联调和既有 30 项隔离浏览器回归通过；原生密码保存提示仍需人工验收 |

WSL 的 Cargo / npm 新依赖缓存放在 `/tmp`，未安装系统或全局工具。两边锁文件分别管理，父仓库原依赖与锁文件未升级。Windows 前端依赖在子仓库内按锁文件安装；WSL 前端原生绑定与 Windows 不互用。

## 原生场景

20 项后台场景实际覆盖：匿名与跨来源控制被拒绝；原生 Include / Match；同批共用 SSH；增量启动保留旧 PID 和已建立通道；并发与计划复用；远程转发、SOCKS 真实数据；参数待应用与删除登记不重启；临时项保存；外部监听冲突；按组停止；网络断线重连；停止取消退避且保留外部 SSH；密码认证；加密密钥生成不泄漏口令；两个跳板分别使用自己的原始配置且不显示伪造延迟；SSH 原文外部编辑冲突；公钥-only 清单；已有实例核验复用和 EOF 清理；认证中 EOF 及时取消。

最初 Linux 辅助运行通过 19 项，当时最终场景为直接 EOF 清理；随后 Windows 最终版增加了已有实例与认证中 EOF 两项，并替换直接 EOF 项，共 20 项。Linux 不作为首版支持平台或 Windows 进程行为的替代证据。

8 项宿主联调覆盖：声明登记；前置未完成时不创建依赖视图；真实 SSH 就绪后打开页面；两个页面共享原连接；隐藏和手动停止后重访不重开；失败保留诊断；等待时打开 Tether 处理密码认证且页面无法调用宿主接口；完整退出取消尚在等待的认证与托管连接，并保留测试中独立启动的 SSH。

所有 SSH 目标均为回环测试服务，密钥和 known_hosts 为新生成的隔离文件。未修改真实 SSH 配置、Agent 服务或其他连接。测试进程仅使用自己创建的句柄/PID 清理，不按名称批量结束进程。

## 已知限制与未完成项

- 原生配置快照支持范围有明确边界。known_hosts 多值路径无法无损区分的空格/相对路径、IPv6 字面量跳板、重复跳板别名、Unix socket 和动态远程转发当前拒绝；不是所有 OpenSSH 语义已验收。自定义 ProxyCommand、CanonicalizeHostname、复杂 token 和 Match exec 尚无完整原生矩阵。
- 主机常用参数表单、有序高级选项、临时覆盖和预览已实现；内部 DEBUG1 用于就绪，目前没有可选日志 quiet 模式。隐藏控制台和不运行远端 shell 均单独处理。
- 密钥主要来自用户登记路径与实际 Agent 输出；尚无默认目录/全部配置引用的自动发现、完整指纹合并和硬件身份专用 UI。没有本地私钥删除或远端授权撤销功能。
- 公钥复制、加密生成、口令/备注工具入口与 Agent 加载/移出已交付；本轮真实验证了生成、公钥读取与清单，未操作真实系统 Agent，也未完整验证修改口令、硬件设备、证书及 Agent 约束。
- SSH 原文编辑保留原文、冲突和备份；显式绑定主配置在写入前经原生校验。独立 Include 片段和 known_hosts 不能冒充主配置做完整验证；没有系统编辑器启动/文件管理器操作。非 UTF-8 文件明确拒绝内置编辑，需外部编辑器。
- 外部观测仅列出可读 SSH 进程及命令行，区分本实例拥有和外部来源；尚无完整监听归属表、规则关联和权限不足的细粒度诊断，未知数据不推断。
- 密码和首次指纹走可见 askpass，已有加密生成场景；真实多因素、硬件触碰、所有 Windows 客户端版本与隐藏窗口表现尚需实机验收。
- Tether 页面按本地偏好或系统选择主题，与 Oriel 使用同一视觉方向；尚未实现随宿主手动主题实时联动。当前停止按钮明确显示组/主机/全部范围，未提供逐端口取消。
- Oriel 的前置引用通过声明或本地配置维护，尚无依赖编辑表单、服务状态管理页面或通用安装更新能力。后台意外退出后需要重新接入，未提供服务自动恢复 UI。
- 同一配置可复用已有后台；完整退出会清理这个实例的全部 Tether 托管项。多个 Oriel 会话共享后台的仲裁、系统挂起/恢复及设备重启尚未实现或验证。
- 最后一轮认证中退出联调末尾出现一次 WebView `PostMessage` 无效窗口句柄诊断；进程退出、待认证取消、托管端口关闭及外部连接保留断言均通过，退出回调日志仍需后续收敛。
- 本轮生成 Tether Release 目录包；Oriel 仅重新构建 Debug 程序，没有重新生成其安装包。未自动 commit、推送或发布。

完整需求和 V01–V33 验收目标保留在[实现交接](docs/implementation-handoff.md)。后续应优先扩展原生配置与认证矩阵、密钥/Agent 工作流及宿主管理可见性，不把缺口写成已交付能力。
