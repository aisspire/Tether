# 实际验证与剩余工作

## 2026-10-24：单条映射复制完整链接命令

- 映射“更多”菜单在“复制规则 ID”下方新增“复制完整链接命令”；后台 `connectionCommand` 只导出该条转发与必要生效参数。已登记项读取当前保存配置，临时项／SSH 来源项读取运行快照；不会复制整组参数、启动连接或依赖 Tether 临时运行配置。
- Windows 输出 PowerShell 命令，已有 WSL／Linux 输出 POSIX shell 命令；保留身份路径与逐跳参数，复杂配置无法回读一致时明确失败。跳板参数内依赖原 ProxyJump 的 `%j`／`%C` token 当前拒绝导出，原有快照语义限制继续适用。
- Windows：`cargo test --locked --offline` 18 项通过，`cargo build --locked --offline` Debug 构建通过；Vue / TypeScript 与 Vite 生产构建通过。WSL／Linux：`cargo test --locked` 21 项和 Debug 构建通过；前端另在隔离临时目录按现有 lockfile 安装项目依赖，生产构建通过。没有更换依赖版本或重写锁文件。
- `npm run test:command` 在 Windows PowerShell 5.1 和 Linux 各 7 项通过：未启动映射只导出一条且生成不连接、复制命令的本地／远程／SOCKS 真实数据、两跳命令在移走原配置后继续工作、同组隔离与保存参数优先、临时项保留本次覆盖且不重读原配置、IPv6 与重复身份文件、特殊跳板 token 拒绝。包含带空格、单引号与 `$` 的路径。服务器、密钥、known_hosts 与数据目录全部隔离，仅连接回环测试端点。
- `npm run test:runtime` 在 Windows 与 Linux 各 25 项通过，包含增量复用、只补缺失、现有连接保留、临时项、认证、重连、外部连接归属、两跳和退出清理。
- `npm run test:interactions` 6 项通过，新增异步命令尚未返回时就申请写入、生成／权限失败保留原因、同时失败不留下未处理拒绝，以及旧浏览器回退；原弹窗与排序事件检查仍通过。
- Windows Edge `npm run test:ui` 26 项通过，覆盖新增菜单位置、单条命令内容、菜单收起、生成／权限失败提示，以及既有主题、窄屏、弹窗、文件、终端观测、参数、计划、拖拽排序与环境切换。剪贴板写入使用测试替身，未读取或改写用户系统剪贴板；不将这些检查表述为所有浏览器权限策略已验收。菜单浅色与深色窄屏截图见测试输出目录。
- 现有 UI 检查修正被弹出菜单遮挡时的切换手势、CDP Enter 字符事件、按动作名称定位按钮、拖拽事件的实际监听容器及重载后等待环境就绪的时序；仅修正测试，不修改原排序／菜单业务。已审查新菜单的浅色与深色窄屏截图。没有构建新的 Release 目录包、安装、发布或自动提交；Windows PowerShell 7 与不同 OpenSSH 版本仍未建立完整矩阵。

## 2026-10-23：拖拽排序与弹窗操作

- 已实现主机、主机内登记映射和计划的手柄排序及 Alt+↑/↓；主列表按 revision 保存数组顺序，编辑弹窗内的映射顺序随草稿提交。排序保留稳定 ID 与引用，不调用连接生命周期动作。长列表拖动时支持页面／弹窗顶部与底部持续滚动，放下、取消、失焦及卸载时清理。
- 所有弹窗采用固定顶部栏与右上角关闭键，确认弹窗增加取消关闭键；窗外关闭统一校验主指针的按下与松开位置，窗内拖到窗外、反向跨边界、内边距点击及取消手势不触发关闭。
- `npm run test:interactions` 4 项通过：完整窗外点击与跨边界保护、取消／指针身份／关闭及卸载清理、稳定 ID 前后排序、页面与弹窗双向边缘滚动及停止清理。事件和滚动检查使用最小模拟宿主，不代表浏览器布局验收。
- Vue / TypeScript 检查、两份交互检查脚本的语法检查及 `git diff --check` 通过。Windows Edge UI 脚本已补充真实鼠标跨边界与未保存保护、长内容关闭键位置、真实 DOM 中的拖动边缘滚动／取消、草稿保存、键盘排序及刷新持久化场景，本轮未运行。
- `npm run build` 在 Vite 阶段因缺少 Linux Rolldown 原生绑定失败；现有 Windows PowerShell 启动报告 `UtilBindVsockAnyPort: socket failed 1`，无法运行 Windows Edge 检查。未安装依赖、修改锁文件或重建交付包；浅深主题与关闭键实际布局仍待 Windows 浏览器验收。

## 2026-10-23：“更多”菜单自动收起

- 主机、映射及密钥的行菜单统一处理外部点击、Escape 关闭与焦点恢复；打开另一菜单会收起旧菜单，选择直接操作项后收起。菜单内输入和公钥详情展开保持可用，普通详情折叠面板不受影响；组件卸载时清理监听。
- `node node_modules/vue-tsc/bin/vue-tsc.js --noEmit`、`node --check scripts/check-ui.mjs` 和 `git diff --check` 通过。
- 已向现有 Windows Edge UI 检查增加外部鼠标点击、菜单切换、键盘激活、Escape 焦点恢复、操作项执行、普通详情保持展开，以及密钥输入、公钥展开和窄屏关闭的回归场景；本轮未执行浏览器交互检查。
- 当前 WSL 环境缺少项目所需的 Linux Rolldown 原生绑定，`npm run build` 在 Vite 阶段失败；尝试启动现有 Windows 程序时出现 `UtilBindVsockAnyPort: socket failed 1`，无法运行 Windows Edge 检查。未安装依赖、改动锁文件或生成交付包。

## 2026-10-23：Windows 打包调用 WSL 的 Cargo 路径

- 从 Windows 复现旧入口失败：默认 Ubuntu 的非交互式 shell 不包含 `~/.cargo/bin`，报 `sh: 1: cargo: not found`；现有 Rust、musl 目标和链接器均可用。
- `build.ps1` 改用随仓库提供的 `scripts/build-linux.sh`，按 PATH、`CARGO_HOME/bin`、`~/.cargo/bin` 查找已有 Cargo；仅调整构建进程环境，脚本以 LF 交付，不修改全局配置或安装工具。
- 从 Windows 运行完整 `build.cmd` 通过，包含锁定依赖的前端生产构建、Windows Release、WSL Linux x64 静态构建及 `target/bundle/` 组包。UI 资源已同步进交付包。
- 隔离模拟检查通过，覆盖含空格的 `CARGO_HOME` 路径、Debug / Release 参数及项目输出目录；`sh -n` 与 `git diff --check` 通过。
- Windows `scripts/check-bundle.mjs` 6 项通过：Linux 静态 ELF、应用声明及随包说明、仓库外前端资源、系统 SSH 检查、CLI 认证状态和 stdin 未关闭时退出。使用隔离数据目录，未运行真实远端连接或宿主联调。

## 2026-10-23：图标悬停说明

- “启动所选”使用纯图标按钮，悬停说明与可访问名称包含所选映射数量；主机展开／收起箭头按当前状态和主机名称显示说明。沿用现有图标样式与操作绑定。
- Windows Vue / TypeScript 检查和 Vite 生产构建通过；静态核对 UI 中的纯图标按钮及主机折叠按钮均声明 `title` 与 `aria-label`。
- 此次局部调整未重跑完整 UI 回归或原生宿主联调；下文记录不作为此次调整后的验证结果。

## 2026-10-23：通透界面与操作层级整理

本节为本轮实际验证；后续较早记录保留为历史。本轮只调整 Tether 页面模板、图标、样式令牌和对应的 UI 验证脚本，没有修改 Rust 后台、公共动作、配置字段或生命周期。

- Windows `npm run build` 通过 Vue / TypeScript 检查与 Vite 生产构建；没有新增或升级依赖，未修改 `package-lock.json`。
- Windows Edge `scripts/check-ui.mjs` 21 项通过，继续覆盖主机与映射编辑、真实回环认证、计划固定成员、密钥文件／Agent 清单、SSH 文件读写保护、终端连接只读关联、删除引用保护、浅深主题、480px 长列表与键盘操作。验证脚本只按新的可访问名称和“主机更多”入口定位操作。
- 已查看本轮生成的浅色主机、主机菜单、计划、计划编辑、密钥、设置、说明，以及深色 480px 主机、主机菜单和编辑弹窗截图。主机菜单在 1180px 与 480px 下均能打开；窄屏菜单改为底部浮层，页面无横向溢出，未捕获浏览器异常。
- 状态与响应时间同时显示文字和颜色；图标按钮有 `aria-label`／`title` 和可见焦点，主要提交动作保留文字。没有根据截图宣称所有分辨率、辅助技术或真实远端场景已经验收。
- 本轮未重跑 Rust 单元、CLI、运行时和宿主联调；这些结果不由本页本轮产生。真实密钥、Agent、远端 SSH 和系统安装边界继续沿用既有限制。

## 2026-10-22：密钥列表、延迟颜色与已知映射的终端连接

本节为本轮实际验证；后文为历史记录，涉及网页入口和外部 SSH 总览的旧结论由本轮替代。

- Windows `npm run build` 通过 TypeScript / Vue 检查与生产构建；没有升级依赖或修改锁文件。
- Windows `cargo clippy --locked --offline --all-targets -- -D warnings` 与 10 项 `cargo test --locked --offline` 通过；覆盖 Windows 命令行引号与转义、显式用户和端口、IPv6 转发、完整端点匹配、方向／目标冲突，以及旧 `openUrl` 配置兼容读取并在保存时丢弃。
- Windows `npm run test:runtime` 23 项通过。新增测试使用临时密钥与回环 SSH2 服务，验证仅返回已有映射的终端连接、不显示无关转发、同端口不同目标不匹配、本地与动态转发核对实际监听、远程项不冒充前置就绪、显式复用不创建托管组、登记变更立即过滤旧关联、停止全部不影响终端连接，以及终端退出后清除关联。其余既有回环转发、认证、断线重连和退出场景通过。
- Windows Edge `npm run test:ui` 18 项通过。新增覆盖延迟结果成功色、自动读取密钥列表、指纹／仅公钥／文件缺失状态与禁用操作、无原始 JSON、密钥窄屏换行，以及实际终端连接在原映射行显示运行来源、启动和停止权限、退出后恢复未启动。既有编辑、计划、引用保护、认证和 480px 浅深布局检查继续通过，未捕获浏览器异常为空。
- 已查看浅色密钥列表、深色 480px 密钥列表和终端连接状态截图。终端连接行显示运行来源，没有新增外部连接总览；未据此宣称所有分辨率及辅助技术完整验收。
- 按现有 `build.ps1` 生成更新后的 Release `target/bundle/`；`npm run test:bundle` 5 项通过，包含随包说明与应用声明、仓库外资源、系统 SSH 检查、认证 CLI 状态和 stdin 未关闭时退出。未安装、发布或创建 Git 提交。
- 文档本地链接、脚本语法及 `git diff --check` 通过。仅修改 Tether 独立仓库，不改变 Oriel 宿主；本轮没有重跑宿主联调。真实远端、系统 Agent 修改、硬件密钥及多因素认证继续保留既有限制。

检测按相同 SSH 目标／别名、显式用户及端口、配置路径和完整转发端点关联，不在轮询中执行 `ssh -G`。配置内隐式转发、不同别名或覆盖方式之间的等价关系、跳板／代理和无法读取的进程不作推断；远程项只确认 SSH 已连接及显式参数，不确认远端监听。密钥清单只读 Agent 信息，本轮未加载或卸载真实 Agent 身份。

## 2026-10-22：主机为中心的映射与计划体验

本节为本轮实际验证；后文既有记录保留为历史，不替代本轮结果。

- Windows `npm run build` 通过 Vue / TypeScript 检查与 Vite 生产构建；按现有 package-lock.json 执行局部 npm ci，未升级依赖或修改锁文件。
- Windows `cargo clippy --locked --offline --all-targets -- -D warnings` 通过；清理 Windows 进程构造分支原有冗余 return，保持进程归属行为。
- Windows `cargo test --locked --offline` 7 项通过，新增覆盖旧配置兼容、稳定引用、可选网页入口及片段地址、非网页／含凭据地址拒绝、目标内用户与 User 覆盖冲突。
- Windows `scripts/check-runtime.mjs` 22 项通过：除既有 20 项真实回环 SSH 行为外，新增草稿主机预览不保存、计划固定成员；同时检查活动连接不能 dismiss、失败提示可关闭及停止后运行记录自动移除。已有实例退出检查改为直接等待 exitCode，避免退出事件已发生时遗漏监听。
- Windows Edge `scripts/check-ui.mjs` 15 项通过：统一主机入口与映射归属、去除重复用户与分类字段、目录／原文共用参数、批量导入预览、重复监听反馈与撤销删除、未保存继续／放弃及键盘保存、外部修订冲突保留草稿、弹窗内真实密码认证、运行参数待应用与明确重启后的实际传输、计划主机分组及固定成员、搜索筛选、主机删除引用清理、480px 深色 40 条长列表和编辑弹窗、折叠记忆及焦点。检查未捕获的浏览器异常为空。
- 已查看浅色主机列表和深色窄屏编辑截图，修正映射行标题窄屏换行。参数、批量编辑和说明弹窗均未出现测试窗口横向溢出；未据此宣称所有分辨率与辅助技术完整验收。
- Oriel 的现有 Windows `scripts/check-tether.mjs` 11 项联调通过，验证原有声明发现、应用布局、前置就绪、共享 PID、认证、手动停止与退出归属。末尾仍有既有 WebView 无效窗口句柄日志，连接清理与外部 SSH 保留断言通过；本轮未修改宿主实现。
- 使用现有 build.ps1 生成 Release `target/bundle/`，最终生产 UI 和 USER_GUIDE.md 同步进包；`scripts/check-bundle.mjs` 5 项通过，涵盖随包声明／说明、仓库外资源、系统 SSH check、CLI 状态及 stdin 仍打开时退出。未安装、提交或发布。
- 文档本地链接与 `git diff --check` 通过。WSL 前端原生绑定与 Windows 不互用，WSL 默认 Cargo 缓存不可写，因此本轮以已有 Windows 工具链完成构建及功能验证；没有安装系统或全局工具。

参数目录按 OpenSSH portable 10.0 文档列出 102 项指令，不是用户本机 Windows 客户端全部参数的逐项验收结果。仅显式原生解析和实际启动确认具体支持情况。真实远端主机、系统 Agent 修改、硬件密钥、多因素及所有客户端版本的历史限制继续保留。

## 2026-10-22：应用布局、发现与随包说明

- Oriel 与 Tether 的 Windows 前端类型检查、生产构建通过；Oriel Windows Debug 和 Release 程序构建通过，Release 使用根目录现有构建入口的 `-NoBundle`，未生成新安装包。
- Oriel 的 26 项 WSL 核心测试通过，Clippy 全目标检查通过；新增覆盖自动发现、额外根目录及缺失目录诊断、路径去重和重复身份、旧模式兼容，以及文档目录边界、大小与 UTF-8 校验。
- Windows `scripts/check-tether.mjs` 11 项联调通过：默认目录发现且不提前启动后台、通用无地址栏布局、宿主说明和焦点恢复、额外读取目录在设置界面保存并保持显式登记，以及原有前置、认证、共享、停止和退出归属。打开说明和保存设置期间原 SSH PID 保持不变。
- Windows `scripts/check-ui.mjs` 6 项通过：独立说明、Escape 焦点恢复、原有主机／规则／计划编辑，以及 480px 深色说明和 40 项长列表。已检查宿主说明和独立深色窄屏截图；主界面的 CDP 截图不包含原生子 WebView，因此另采集实际工具页面图像。
- 现有构建脚本生成 Tether Debug 目录包，说明、声明和 UI 一起交付；5 项 bundle 冒烟通过，包含从仓库外工作目录读取资源和随包说明、OpenSSH 检查及退出。未发布或安装。
- 既有浏览器 30 项回归通过；原生密码保存及再次填充仍需人工验收。首次鼠标调宽检查超时，记录指针事件后复测通过，没有据此改动业务布局。
- 联调退出末尾仍出现既有的 WebView 无效窗口句柄日志，连接取消、清理和外部 SSH 保留断言通过。本轮未改变 SSH 协议实现、认证范围或多宿主仲裁限制。

以下保留 2026-10-21 的实现与验收记录，不视为本轮重新覆盖了所有后台场景。

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
- 外部 SSH 总览已取消，改为只为已有映射关联终端连接；当前检测范围及限制见本页最新验证记录。
- 密码和首次指纹走可见 askpass，已有加密生成场景；真实多因素、硬件触碰、所有 Windows 客户端版本与隐藏窗口表现尚需实机验收。
- Tether 页面按本地偏好或系统选择主题，与 Oriel 使用同一视觉方向；尚未实现随宿主手动主题实时联动。当前停止按钮明确显示组/主机/全部范围，未提供逐端口取消。
- Oriel 的前置引用通过声明或本地配置维护，尚无依赖编辑表单、服务状态管理页面或通用安装更新能力。后台意外退出后需要重新接入，未提供服务自动恢复 UI。
- 同一配置可复用已有后台；完整退出会清理这个实例的全部 Tether 托管项。多个 Oriel 会话共享后台的仲裁、系统挂起/恢复及设备重启尚未实现或验证。
- 最后一轮认证中退出联调末尾出现一次 WebView `PostMessage` 无效窗口句柄诊断；进程退出、待认证取消、托管端口关闭及外部连接保留断言均通过，退出回调日志仍需后续收敛。
- 本轮生成 Tether Release 目录包；Oriel 仅重新构建 Debug 程序，没有重新生成其安装包。未自动 commit、推送或发布。

完整需求和 V01–V33 验收目标保留在[实现交接](docs/implementation-handoff.md)。后续应优先扩展原生配置与认证矩阵、密钥/Agent 工作流及宿主管理可见性，不把缺口写成已交付能力。
