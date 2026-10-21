# Tether

Windows 原生 OpenSSH 转发管理工具，使用 Vue 3 / TypeScript 页面和 Rust 本地后台。可以独立运行，也能通过 Oriel 的版本 2 声明接入；源码、构建、依赖及业务配置均独立维护。

启动只补缺少的转发，同批兼容项共用 SSH；停止按连接组、主机或全部托管连接处理。匹配端口或规则不会获得外部进程的管理权。

## 构建与独立运行

需要已有 Windows Rust 1.85+（MSVC 工具链）、Node 22.12+、npm 10，以及系统 OpenSSH 的 `ssh`、`ssh-keygen`、`ssh-add`。缺少工具时自行准备；构建与启动脚本不安装系统组件或启用 Agent 服务。

在本仓库的 Windows PowerShell 中执行：

```powershell
.\build.cmd
.\target\bundle\tether.exe check
.\target\bundle\tether.exe serve
```

`build.cmd` 调用同目录的 `build.ps1` 并透传参数，CMD 与 PowerShell 均可使用；例如 `build.cmd -DebugBuild`。从 Oriel 父仓库可执行 `plugins\external\tether\build.cmd`，脚本会自动定位 Tether 目录。父仓库根目录的 `build.cmd` 只打包 Oriel，不会打包或安装 Tether。

构建按 `package-lock.json` 执行局部 `npm ci`，按 `Cargo.lock` 构建 Release，输出 `target/bundle/`：`tether.exe`、`dist/`、`plugin.json`、`oriel.ps1`。可将整个目录放在其他位置。`-DebugBuild` 可生成调试包。不创建安装程序，也不发布。

`serve` 输出一行 JSON，包含实际后台身份和 `pageUrl`。在浏览器打开该地址即可添加主机、规则和计划；后台运行期间可关闭页面，转发继续。终端按 Ctrl+C 退出后台会停止托管连接。只监听随机 IPv4 回环端口，不改变系统代理。

默认数据目录为 `%LOCALAPPDATA%/Tether/`，可用 `--data-dir C:/absolute/path` 或进程环境变量 `TETHER_DATA_DIR` 隔离。目录内 `config.json` 是规则与计划的权威；`instance.json` 含敏感的本机调用身份，不应共享。原有 SSH 文件、私钥和 known_hosts 继续由 OpenSSH 使用。

开发时可执行 `npm ci --ignore-scripts`、`npm run build`、`cargo build --locked`。直接运行开发可执行文件时设置 `TETHER_UI_DIR` 为本仓库 `dist` 的绝对路径，或把 `dist` 放在可执行文件旁。`npm run dev` 仅提供前端开发服务器，不能代替带身份认证的后台服务。

## 已实现范围

- 本地、远程、动态 SOCKS 转发，稳定规则 ID、临时项保存、批量计划和显式自动启动计划。
- 增量启动、兼容分组、并发去重、部分失败、参数待应用、运行快照、按组/主机/全部停止。
- 原生 Include / Match 解析、有序 `-o`、跳板链、多个身份文件、临时覆盖和生效预览；配置内 TCP 转发参与增量规划，不自动改写原文件。
- 保活、断线退避重连、认证失败暂停；停止及退出取消重连。Windows 在恢复 SSH 前将其加入 Job Object，清理实际拥有的子进程。
- 基于 SSH 自身的认证、监听及远程确认事件判断就绪；不以外部端口或进程存在代替就绪。
- 页面认证提示：首次主机确认、密码、加密密钥；回答只驻留内存，可取消和超时。
- 配置版本校验、冲突检测及原子保存；SSH 原文编辑、外部修改检测、备份及绑定主配置的原生校验。
- 登记密钥路径、公钥和指纹、只含公钥与 Agent-only 身份区分、加密密钥生成、公钥复制、Agent 加载/移出；口令与备注修改提供公共动作。
- Windows SSH 命令行只读观测；主机处单一 TCP 响应时间，跳板目标显示无数据。
- 同一套紧凑页面、浅深主题、主机常用参数表单、计划和配置编辑；Oriel 前置等待、共享、认证及退出接入。

## CLI 与 Oriel

```powershell
.\tether.exe request --json '{"action":"status"}'
.\tether.exe request --json '{"action":"ensure","request":{"ruleIds":["your-stable-rule-id"],"consumer":"manual"}}'
.\tether.exe request --json '{"action":"stop","scope":"all"}'
```

PowerShell 不同版本对原生命令 JSON 引号的处理不同；自动化调用优先使用下面的 stdio 契约。所有命令均可加 `--data-dir`，仅操作该实例。

在支持 `local-service-v1` 的 Oriel 中登记交付目录即可。`oriel.ps1` 仅包装 `serve --oriel-stdio`；不会自动安装或更新。依赖页面通过 `prerequisites: [{"pluginId":"tether","ruleId":"..."}]` 引用规则。宿主确认就绪后创建页面；切换、隐藏和关闭页面保留转发，完整退出则清理接入实例的全部 Tether 托管转发。

同一配置已有后台时，stdio 接入核验实例后复用；因此 Oriel 退出也会清理该后台内手动与计划启动的托管连接。外部 SSH 和 Agent 不受影响。当前未实现多个 Oriel 同时共享一个 Tether 实例的会话仲裁。

准确输入输出、配置示例与错误边界见[实现契约](docs/implementation.md)。设计入口见[当前基线](docs/design/current-design.md)，需求验收矩阵见[实现交接](docs/implementation-handoff.md)。

## 验证与限制

```powershell
cargo test --locked
npm run build
npm run test:runtime
npm run test:ui
```

原生检查使用临时密钥、受控回环 SSH2 服务、隔离目录与测试浏览器配置，不连接真实远端。运行检查前设置 `CARGO_TARGET_DIR=target/windows` 并构建 Windows Debug 程序。测试仅对自己新建的目录设置 ACL。Linux 可运行后台辅助验证，但不作为产品平台承诺。

本轮具体结果、尚未支持的配置语义和未完成能力见[验证与缺口记录](verification.md)。没有硬件身份、多因素、系统 Agent 服务及所有客户端版本的完整验收；不把单元测试或网页截图当成这些能力已通过。
