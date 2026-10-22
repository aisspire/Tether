// Parameter names: OpenSSH portable V_10_0_P1 ssh_config.5. Labels and controls are maintained by Tether.
// https://github.com/openssh/openssh-portable/blob/V_10_0_P1/ssh_config.5
export interface SshOption {name:string;label:string;restricted?:string;choices?:string[];repeatable?:boolean;numeric?:boolean}
export const sshOptions:SshOption[] = [
  {
    "name": "AddKeysToAgent",
    "label": "自动加载密钥到 Agent"
  },
  {
    "name": "AddressFamily",
    "label": "地址族",
    "choices": [
      "any",
      "inet",
      "inet6"
    ]
  },
  {
    "name": "BatchMode",
    "label": "禁用交互式认证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "BindAddress",
    "label": "本机出口地址"
  },
  {
    "name": "BindInterface",
    "label": "本机出口网卡"
  },
  {
    "name": "CASignatureAlgorithms",
    "label": "证书机构签名算法"
  },
  {
    "name": "CanonicalDomains",
    "label": "主机名补全域"
  },
  {
    "name": "CanonicalizeFallbackLocal",
    "label": "主机名补全失败时回退",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "CanonicalizeHostname",
    "label": "主机名规范化",
    "choices": [
      "no",
      "yes",
      "always"
    ]
  },
  {
    "name": "CanonicalizeMaxDots",
    "label": "主机名补全点数上限",
    "numeric": true
  },
  {
    "name": "CanonicalizePermittedCNAMEs",
    "label": "允许的规范名称转换"
  },
  {
    "name": "CertificateFile",
    "label": "用户证书文件",
    "repeatable": true
  },
  {
    "name": "ChannelTimeout",
    "label": "通道空闲超时"
  },
  {
    "name": "CheckHostIP",
    "label": "核对主机 IP",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "Ciphers",
    "label": "加密算法"
  },
  {
    "name": "ClearAllForwardings",
    "label": "清除全部转发",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "Compression",
    "label": "数据压缩",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "ConnectTimeout",
    "label": "SSH 连接超时",
    "numeric": true
  },
  {
    "name": "ConnectionAttempts",
    "label": "连接尝试次数",
    "numeric": true
  },
  {
    "name": "ControlMaster",
    "label": "连接复用主进程",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "ControlPath",
    "label": "连接复用控制路径",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "ControlPersist",
    "label": "保持复用主进程",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "DynamicForward",
    "label": "动态端口映射",
    "restricted": "请在主机的端口映射中配置",
    "repeatable": true
  },
  {
    "name": "EnableEscapeCommandline",
    "label": "启用交互转义命令",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "EnableSSHKeysign",
    "label": "启用主机认证签名工具",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "EscapeChar",
    "label": "交互转义字符",
    "restricted": "与 Tether 的托管或交互边界冲突"
  },
  {
    "name": "ExitOnForwardFailure",
    "label": "转发失败时退出",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "FingerprintHash",
    "label": "指纹摘要算法",
    "choices": [
      "sha256",
      "md5"
    ]
  },
  {
    "name": "ForkAfterAuthentication",
    "label": "认证后转入后台",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "ForwardAgent",
    "label": "转发认证 Agent",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "ForwardX11",
    "label": "转发 X11",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "ForwardX11Timeout",
    "label": "X11 转发超时"
  },
  {
    "name": "ForwardX11Trusted",
    "label": "信任 X11 转发",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "GSSAPIAuthentication",
    "label": "GSSAPI 认证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "GSSAPIDelegateCredentials",
    "label": "委派 GSSAPI 凭据",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "GatewayPorts",
    "label": "允许其他设备访问本地转发",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "GlobalKnownHostsFile",
    "label": "系统主机指纹文件"
  },
  {
    "name": "HashKnownHosts",
    "label": "对主机指纹条目名称散列",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "Host",
    "label": "主机匹配条件",
    "restricted": "请在原始 SSH 配置文件中维护"
  },
  {
    "name": "HostKeyAlgorithms",
    "label": "服务器主机密钥算法"
  },
  {
    "name": "HostKeyAlias",
    "label": "主机指纹匹配别名"
  },
  {
    "name": "HostbasedAcceptedAlgorithms",
    "label": "主机认证签名算法"
  },
  {
    "name": "HostbasedAuthentication",
    "label": "基于主机的认证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "Hostname",
    "label": "实际 SSH 主机名"
  },
  {
    "name": "IPQoS",
    "label": "网络服务质量"
  },
  {
    "name": "IdentitiesOnly",
    "label": "仅使用指定身份",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "IdentityAgent",
    "label": "认证 Agent 地址"
  },
  {
    "name": "IdentityFile",
    "label": "身份文件",
    "repeatable": true
  },
  {
    "name": "IgnoreUnknown",
    "label": "忽略指定未知参数"
  },
  {
    "name": "Include",
    "label": "包含其他配置文件",
    "restricted": "请在原始 SSH 配置文件中维护"
  },
  {
    "name": "KbdInteractiveAuthentication",
    "label": "键盘交互认证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "KbdInteractiveDevices",
    "label": "键盘交互认证设备"
  },
  {
    "name": "KexAlgorithms",
    "label": "密钥交换算法"
  },
  {
    "name": "KnownHostsCommand",
    "label": "外部主机指纹命令",
    "restricted": "与 Tether 的托管或交互边界冲突"
  },
  {
    "name": "LocalCommand",
    "label": "本地执行命令",
    "restricted": "与 Tether 的托管或交互边界冲突"
  },
  {
    "name": "LocalForward",
    "label": "本地端口映射",
    "restricted": "请在主机的端口映射中配置",
    "repeatable": true
  },
  {
    "name": "LogLevel",
    "label": "日志级别",
    "restricted": "Tether 使用此项判断连接就绪并保留诊断信息"
  },
  {
    "name": "LogVerbose",
    "label": "详细日志位置",
    "restricted": "Tether 使用此项判断连接就绪并保留诊断信息"
  },
  {
    "name": "MACs",
    "label": "消息认证算法"
  },
  {
    "name": "Match",
    "label": "条件匹配",
    "restricted": "请在原始 SSH 配置文件中维护"
  },
  {
    "name": "NoHostAuthenticationForLocalhost",
    "label": "本机连接免主机验证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "NumberOfPasswordPrompts",
    "label": "密码提示次数",
    "numeric": true
  },
  {
    "name": "ObscureKeystrokeTiming",
    "label": "隐藏键入时间特征"
  },
  {
    "name": "PKCS11Provider",
    "label": "PKCS11 身份提供程序"
  },
  {
    "name": "PasswordAuthentication",
    "label": "密码认证",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "PermitLocalCommand",
    "label": "允许执行本地命令",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "PermitRemoteOpen",
    "label": "动态远程转发允许目标"
  },
  {
    "name": "Port",
    "label": "SSH 端口",
    "numeric": true
  },
  {
    "name": "PreferredAuthentications",
    "label": "认证方法顺序"
  },
  {
    "name": "ProxyCommand",
    "label": "代理连接命令"
  },
  {
    "name": "ProxyJump",
    "label": "跳板链"
  },
  {
    "name": "ProxyUseFdpass",
    "label": "代理通过文件描述符传递连接",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "PubkeyAcceptedAlgorithms",
    "label": "用户公钥签名算法"
  },
  {
    "name": "PubkeyAuthentication",
    "label": "公钥认证",
    "choices": [
      "yes",
      "no",
      "unbound",
      "host-bound"
    ]
  },
  {
    "name": "RekeyLimit",
    "label": "重新交换密钥限制"
  },
  {
    "name": "RemoteCommand",
    "label": "远端执行命令",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "RemoteForward",
    "label": "远程端口映射",
    "restricted": "请在主机的端口映射中配置",
    "repeatable": true
  },
  {
    "name": "RequestTTY",
    "label": "请求远端终端",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "RequiredRSASize",
    "label": "RSA 密钥最小位数",
    "numeric": true
  },
  {
    "name": "RevokedHostKeys",
    "label": "撤销的主机密钥文件"
  },
  {
    "name": "SecurityKeyProvider",
    "label": "硬件密钥提供程序"
  },
  {
    "name": "SendEnv",
    "label": "传递环境变量名称",
    "repeatable": true
  },
  {
    "name": "ServerAliveCountMax",
    "label": "保活未响应次数上限",
    "numeric": true
  },
  {
    "name": "ServerAliveInterval",
    "label": "保活间隔",
    "numeric": true
  },
  {
    "name": "SessionType",
    "label": "SSH 会话类型",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "SetEnv",
    "label": "设置远端环境变量",
    "repeatable": true
  },
  {
    "name": "StdinNull",
    "label": "忽略标准输入",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "StreamLocalBindMask",
    "label": "Unix 套接字权限掩码"
  },
  {
    "name": "StreamLocalBindUnlink",
    "label": "删除已有 Unix 套接字",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "StrictHostKeyChecking",
    "label": "主机指纹校验方式",
    "choices": [
      "ask",
      "yes",
      "accept-new",
      "no"
    ]
  },
  {
    "name": "SyslogFacility",
    "label": "日志设施"
  },
  {
    "name": "TCPKeepAlive",
    "label": "TCP 保活",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "Tag",
    "label": "配置匹配标签"
  },
  {
    "name": "Tunnel",
    "label": "网络隧道类型",
    "restricted": "由 Tether 管理进程、认证交互或转发就绪行为"
  },
  {
    "name": "TunnelDevice",
    "label": "网络隧道设备"
  },
  {
    "name": "UpdateHostKeys",
    "label": "更新服务器主机密钥"
  },
  {
    "name": "User",
    "label": "登录用户覆盖"
  },
  {
    "name": "UserKnownHostsFile",
    "label": "用户主机指纹文件"
  },
  {
    "name": "VerifyHostKeyDNS",
    "label": "DNS 主机指纹验证",
    "choices": [
      "no",
      "yes",
      "ask"
    ]
  },
  {
    "name": "VersionAddendum",
    "label": "版本附加文本"
  },
  {
    "name": "VisualHostKey",
    "label": "显示主机密钥图形",
    "choices": [
      "yes",
      "no"
    ]
  },
  {
    "name": "XAuthLocation",
    "label": "X11 认证工具路径"
  }
]
