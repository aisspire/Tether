# 实现契约 v1

本文描述 0.1.0 已存在的接口。产品要求仍见[当前基线](design/current-design.md)，验证见[实际记录](../verification.md)。

## 配置

`config.json` 版本 1，保存前校验 ID、引用、端口、参数和版本；损坏文件不覆盖。读取版本摘要 `revision` 后提交修改，外部变化需重新加载。规则稳定 ID 与名称、备注、顺序分离；删除登记不停止连接。

```json
{
  "version": 1,
  "hosts": [{
    "id": "host-prod", "name": "生产", "destination": "prod",
    "ssh": "ssh", "configFile": "C:/Users/you/.ssh/config",
    "options": ["ServerAliveInterval=15", "IdentityFile=C:/keys/id_ed25519"],
    "reconnect": true, "hidden": true, "timeoutSecs": 60,
    "notes": "", "category": ""
  }],
  "rules": [{
    "id": "ops-web", "name": "管理页面", "hostId": "host-prod",
    "direction": "local", "bind": "127.0.0.1", "port": 18080,
    "target": "127.0.0.1", "targetPort": 8080, "notes": ""
  }],
  "plans": [{"id":"daily","name":"日常","ruleIds":["ops-web"],"autoStart":false}],
  "keyFiles": [], "sshFiles": [], "theme": "system"
}
```

网页快捷入口已移除。旧版本 1 配置中的 `openUrl` 仅兼容读取，忽略其内容，后续保存不再写出，不改 ID 或计划引用。`category` 为兼容旧配置保留，界面不再提供主机分类。

方向为 `local` / `remote` / `dynamic`；动态转发不需要目标。IPv6 地址不带方括号，运行编译时加括号。空或通配监听需由用户明确选择。`configFile: null` 使用原生默认配置；`none` 可选择无用户配置。高级 `options` 每项为一份原生 `-o Name=Value`，保持顺序与重复项；路径含空格时按 SSH 配置语法给值加双引号。

运行层固定仅转发、非后台化、无控制 socket、无远端命令、退出失败策略与就绪所需日志；冲突选项在校验时拒绝。内部 DEBUG1 用于就绪与诊断，目前没有用户可选的日志 quiet 模式，不与隐藏控制台混为一项。

界面的主机、映射和计划顺序使用对应配置数组顺序，不新增版本字段；排序保存仍校验 revision，且不调用启动、停止或重连。主机编辑内的映射排序属于草稿，保存时统一提交。

## 运行规划

启动显式调用本机 `ssh -G`，因此可能触发原文件的 `Match exec`；不在轮询里自动解析。按实际生效配置构建去除所有来源转发的临时配置，再由同一客户端回读核对。每一跳分别解析其原始配置，目标的身份覆盖不会套到跳板。仅将缺少的 TCP 转发加入新进程。

继承的转发 ID 为 `ssh:<host-id>:<语义摘要>`，标为只读来源。Unix socket 和动态远程转发等无法可靠表达的来源明确失败。存在歧义的 known_hosts 多值路径、重复跳板别名、IPv6 字面量跳板等不静默降级。原文件不被规划器重写。

成功停止并清理后从 `status.groups` 移除运行记录，不保留 stopped 历史。失败保留诊断直到 dismiss 或同一映射重试；dismiss 不得替代停止。UI 映射登记与运行结果合并展示，运行参数与日志按实际连接展开。

连接参数变化返回 `pending`，用户明确停止受影响连接组，再启动才应用。重连沿用该组快照；外部文件内容仍可能变化。启动新组失败不回滚其他组。组拥有实际进程句柄及 Windows Job Object，运行日志最多保留 160 行、仅在内存中；旧 PID 和端口扫描从不作为接管依据。

## 公共动作

后台 HTTP `POST /api` 只允许当前回环 Host、同来源或无 Origin，以及实例 Bearer 身份。拒绝匿名、跨来源、DNS rebinding Host；不提供 CORS。页面从 URL fragment 读取身份后移入当前页签 sessionStorage，移除 fragment。认证 helper 使用另一个端口和另一份身份，页面不能制造认证提示。GET 页面接口没有业务执行能力。

请求 `{"action":"...", ...}`，结果为 `{"ok":true,"value":...}` 或 `{"ok":false,"error":"..."}`。

| 动作 | 输入与语义 |
| --- | --- |
| status / reload | 读取状态 / 显式重载本地配置；不启动连接 |
| save | `config`、`revision`；成功后才更新内存 |
| preview / test | `hostId` 或完整 `host` 草稿、可选 `overrides`；生效参数 / 目标 TCP 响应，不保存草稿 |
| connectionCommand | `ruleId`、临时项可选 `groupId`；返回 `{program,args,command,shell}`，只生成当前映射的终端命令，不启动或复用连接 |
| dismiss | `groupId`；只移除已结束且无进程、无重连意图的失败记录，拒绝活动连接 |
| ensure | `request: {ruleIds?, temporary?, overrides?, planId?, consumer?}` |
| stop | `scope: group/host/all`，前两者需 `id` |
| shutdown | 拒绝新动作、取消认证与重连、停止本实例托管资源并退出 |
| answer / cancelPrompt | `id`、`answer` / `id`；认证回答 / 取消；不持久保存回答 |
| adopt | `groupId`、与运行意图相同的 `rule`、`revision`；保存且不重启 |
| files.read / files.write | 已登记的 `path`；写入还需 `text`、`revision`，创建备份 |
| keys.list / keys.public | 文件与 Agent 清单 / 已登记 `path` 的公钥 |
| keys.generate | 已登记且不存在的 `path`，`algorithm: ed25519/rsa/ecdsa`；交互输入口令 |
| keys.passphrase / keys.comment | 已登记私钥 `path`，备注还需 `comment`；口令仍通过交互 |
| agent.add / agent.remove | 已登记 `path`；不启用或停止系统 Agent 服务 |
| knownHosts.find / knownHosts.remove | 已登记文件 `path`、`host`；删除须 `confirm: true` |

`ensure` 返回 `{instance, ready, results}`；每项含 `ruleId`、可选 `groupId`、`state`、`reused` 和 `error`。只有本次所有规则都确认 `running` 时 `ready: true`。`failed`、`pending`、`conflict`、`stopped`、`timeout`、`unverified` 均不满足前置依赖。资源等待不检测远端业务 HTTP 健康。

## 单条映射完整链接命令（2026-10-24）

`connectionCommand` 通过现有环境会话路由，读取当前环境的已登记规则及其已保存主机；规则仍登记时不使用旧组参数或页面里的本次启动覆盖。未登记但仍有运行记录的临时项／SSH 配置来源项使用所属组的规则及快照，传入 `groupId` 可指定该组；已消失的引用返回错误。

导出默认以 `ssh -N -T` 开头，使用 `-F none`、仅一项 `-L`／`-R`／`-D` 与必要连接参数。基于同一 OpenSSH 客户端的无配置默认值生成差异参数，再以 `ssh -G` 回读核对非转发参数，保留有序重复身份项、IPv6 和路径引号。端口和用户使用 `-p`、`-l`，无延迟展开且文件存在的身份路径使用 `-i`；其他身份路径仍保留 `-o IdentityFile=...`，避免 `-i` 提前展开 `~` 或丢弃不存在的身份。其他选项使用 `-o key=value`。内部就绪日志级别改为普通 `INFO`；保活和仅转发设置保留。Agent socket 已绑定且未显式指定其他 IdentityAgent 时写入对应参数，不导出问答口令、私钥内容或后台调用身份。

ProxyJump 按已有逐跳快照展开为嵌套 `ProxyCommand`，每跳保留自己的参数，以 `-W` 转递上层连接，并单独设置标准输入与清除转发；不依赖组的临时文件，不将目标身份覆盖套给跳板。由于改写会改变 `%j` 与 `%C` 的含义，跳板上下文内使用这些 token 的参数明确拒绝。原有配置快照的不支持边界仍适用；生成可能显式解析原始配置，因此仍可能执行用户原有 `Match exec`，不在状态轮询中运行。

`shell` 为 `PowerShell` 或 `POSIX`。Windows 直接调用 SSH，可安全裸写的参数不加引号，其余用 PowerShell 字面量单引号；自定义程序路径需要引号时使用调用运算符 `&`。嵌套双引号参数使用 `ssh -N -T --% ...` 保留原生引号，不生成进程启动包装。`--%` 仍可能展开 `%名称%` 环境引用，因此复杂引号模式下存在非相邻或多组百分号时明确拒绝，避免改变路径或命令；语义依据见 [PowerShell 参数解析](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_parsing?view=powershell-5.1)。WSL／Linux 用 POSIX shell 引号，按所属环境执行，不额外包装 `wsl.exe`。命令中的密钥、Agent 和用户自定义 ProxyCommand 等仍依赖该环境现有资源，不承诺跨机器移植。无法回读一致、包含换行／空字符或超出 Windows 长度边界时不返回可复制命令。

前端在点击时即向 Clipboard API 提供待生成文本，避免等待 SSH 解析后丢失用户手势；支持时使用异步 `ClipboardItem`，否则回退 `writeText`。生成与权限失败均显示可关闭错误，不显示本次成功通知；处理期间禁止重复复制。

## Oriel stdio-v1

`oriel.ps1 [-DataDir ABSOLUTE]` 前台运行后台。stdout 首行：`{protocol:1, ready:true, instance, origin, token, pageUrl}`。随后每行一个公共动作 JSON，每行返回一个结果信封；动作有序执行。认证/状态可由同一实例的 HTTP 页面并行处理。stdin EOF 独立监测，即使正在等待认证也立即启动退出清理。

已有后台时先核验实例身份，再桥接公共接口。后续 status 的 instance 必须与首行一致。Oriel 对真实状态和规则结果分别核对；脚本退出不算转发就绪。正常退出通过已验证实例的受认证接口取消待办，再结束 stdio 包装进程；不按名称全局结束 SSH。

宿主声明版本 2：`capabilities: ["web-page","local-service-v1"]`，`lifecycle: {protocol:"stdio-v1", entrypoint:"oriel.ps1", args:[]}`，页面 `service: "tether"`。其他纯页面可只声明 `web-page` 并添加 `prerequisites`。协议不包含任意脚本、安装更新或通用工作流动作。

## 应用展示与随包说明（2026-10-22）

`plugin.json` 的入口新增 `presentation: "application"`，供支持该扩展的 Oriel 选择通用工具布局，不显示浏览器地址栏和通用前进后退；不改变原有服务、前置等待及退出契约。顶层 `documentation: "USER_GUIDE.md"` 指向交付目录内的说明文件，宿主读取前检查路径、文件类型、UTF-8 和 256 KiB 大小边界，按文本显示，不执行文档 HTML。

构建将同一份 [USER_GUIDE.md](../USER_GUIDE.md) 编入独立 UI，并原样复制进交付目录；独立页面有说明弹窗，不额外依赖 Oriel API。交付文件夹可放入宿主默认或额外插件读取目录，保存设置或重新加载时自动发现，也兼容显式登记路径。新增宿主能力需要相应版本的 Oriel，此后兼容工具可复用声明而不修改宿主路由。

## 主机操作界面（2026-10-22）

主机内维护连接配置和多条映射，批量保存沿用 save + revision 的配置原子写入及外部修改检测；计划按主机选中并保存明确 ruleIds，不自动纳入新增映射。删除主机／映射时同步移除其计划引用，但运行资源只按用户选择停止；保留的连接继续作为临时托管项展示。

常用参数、目录覆盖行和原文共用有序 options。目录固定为 OpenSSH portable 10.0 文档的 102 项，不代表本机可用清单；原生 preview 和启动负责最终语义校验，目标内用户与 User 覆盖冲突由后台拒绝。映射批量导入支持制表符和 JSON，预览时检查字段及同主机监听冲突，保存前统一校验。

启动结果提供逐项状态及仅重试未就绪的已登记映射，重试沿用该次请求的 overrides；成功通知短暂显示，错误和失败记录由用户关闭。编辑草稿绑定打开时的 revision，保存失败保留输入；设置原文绑定读取的文件路径与 revision，切换路径不允许把旧内容写入新文件。

## 已知映射的终端连接检测（2026-10-22）

`status` 读取触发后台最多每三秒一次的 Windows 只读检查，返回 `externalMatches: [{ruleId,pid,verified}]` 及可选 `externalCheckError`；不返回无关进程清单或原始命令行。仅匹配登记规则和托管列表内的临时规则；配置变更立即按当前规则和主机过滤旧关联，后续检查再更新进程／监听状态。状态读取不等待扫描完成，认证和界面轮询不受扫描阻塞。

只解析显式转发及足够一致的目标、用户／端口和配置路径，不运行原生配置解析；不可靠地还原 config 内转发、跳板／代理、控制复用或不同别名之间的等价关系。本地／动态项需同一 PID 的已建立连接和监听，`verified: true`；远程项仅标识连接及参数，`verified: false`，并在显式 ensure 时返回 `unverified`。仅经过监听核对的项能返回 `{state:"running",reused:true,source:"external",pid}`，不建立托管组；停止和退出不影响它。

密钥 `keys.list.files` 补充已验证格式的 `publicKey`，只读公钥文件，用于关联 Agent 身份；不返回私钥内容。界面自动读取并以文件列表及独立 Agent 区展示，支持搜索与按文件操作，不再显示 JSON。
