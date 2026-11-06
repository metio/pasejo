# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## 接收者

recipient-added = 已添加接收者 '{ $public_key }'
recipient-removed = 已移除接收者 '{ $public_key }'
recipient-does-not-exist-ignored = 接收者 '{ $public_key }' 在存储中不存在 - 忽略
recipient-unsupported-key-skipped = 跳过 { $provider } 用户 '{ $username }' 的 '{ $key_type }' 类型密钥：age 无法向其加密

## 密文

secret-added = 已在 '{ $secret_path }' 添加密文
secret-edited = 已在 '{ $secret_path }' 编辑密文
secret-generated = 已在 '{ $secret_path }' 生成密文
secret-copied = 已将密文从 '{ $source_path }' 复制到 '{ $target_path }'
secret-moved = 已将密文从 '{ $source_path }' 移动到 '{ $target_path }'
secret-removed = 已移除 '{ $secret_path }' 的密文
secret-show-as-qrcode = 以二维码显示 '{ $secret_path }' 的密文
secret-show-as-text = 以文本显示 '{ $secret_path }' 的密文
secret-copy-into-clipboard = 密文 '{ $secret_path }' 已复制到剪贴板 — 将在 { $duration } 后清除（按 Ctrl-C 立即清除）

## 一次性密码

one-time-password-added = 已在 '{ $password_path }' 添加一次性密码
one-time-password-copied = 已将一次性密码从 '{ $source_path }' 复制到 '{ $target_path }'
one-time-password-moved = 已将一次性密码从 '{ $source_path }' 移动到 '{ $target_path }'
one-time-password-removed = 已移除 '{ $password_path }' 的一次性密码
one-time-password-show = 显示 '{ $password_path }' 的一次性密码
one-time-password-copy-into-clipboard = 一次性密码 '{ $password_path }' 已复制到剪贴板 — 将在 { $duration } 后清除（按 Ctrl-C 立即清除）

## 身份

identity-added = 已添加使用文件 '{ $identity_file }' 的身份
identity-removed = 已移除使用文件 '{ $identity_file }' 的身份
no-identities-exist-yet = 存储 '{ $store_name }' 中尚未有身份。请使用 'pasejo identity add ...' 添加。

## 存储

store-add-success = 已在 '{ $store_path }' 添加存储 '{ $store_name }'
store-set-default = 存储 '{ $store_name }' 现在是默认存储
store-remove-success = 已移除存储 '{ $store_name }'
store-remove-success-with-data = 已移除存储 '{ $store_name }' 并删除其加密数据文件

## 钩子执行

execute-pull-hooks = 正在执行存储 '{ $store_name }' 的拉取钩子
execute-push-hooks = 正在执行存储 '{ $store_name }' 的推送钩子
hook-run-nothing-selected = 没有可执行的内容：请使用 --pull 和/或 --push 选择要执行的钩子。
export-plaintext-warning = 警告：导出内容包含所有明文密钥。
export-hotp-omitted = 警告：Bitwarden 无法存储 HOTP 一次性密码，因此以下条目不包含在导出中：{ $names }
export-orphaned-otp-omitted = 警告：以下一次性密码没有同名的密文，不包含在导出中：{ $names }
export-written-to-file = 已将导出写入 '{ $path }'

## 合并冲突

merge-conflict-recipient-names = 公钥 '{ $public_key }' 的接收者合并冲突：名称 '{ $first_name }' 与 '{ $second_name }' 不同
merge-conflict-recipient-removed-and-renamed = 公钥 '{ $public_key }' 的接收者合并冲突：一个版本中已移除，另一个版本中重命名为 '{ $new_name }'
merge-conflict-values = '{ $secret_path }' 的 { $value_type } 合并冲突：两个版本中的值不同
merge-conflict-removed-and-modified = '{ $secret_path }' 的 { $value_type } 合并冲突：一个版本中已移除，另一个版本中已修改

## 列表输出 (stdout)

list-global-identity = 全局: { $identity_file }
list-store-identity = 存储: { $identity_file }
list-store-identity-also-global = 存储: { $identity_file }（也已全局注册）
list-global-pull-hook = 全局拉取: { $command }
list-global-push-hook = 全局推送: { $command }
list-store-pull-hook = 存储拉取: { $command }
list-store-push-hook = 存储推送: { $command }
list-store = { $is_default ->
    [true] { $store_name }: { $store_path }（默认）
   *[false] { $store_name }: { $store_path }
  }
password-strength = { $secret_path }: { $score }/100
secret-search-match =
    { $key }:
    { $value }

## 剪贴板

clipboard-read-for-compare-failed = 读取剪贴板进行比较失败: { $error }
clipboard-ctrlc-handler-install-failed = 安装 Ctrl-C 处理程序失败: { $error }。剪贴板将仅在配置的超时后清除。
clipboard-clear-failed = 清除剪贴板失败: { $error }
clipboard-manual-clear-required = 剪贴板无法自动清除 — 请立即手动清除。
clipboard-notification-dispatch-failed = 显示剪贴板清除通知失败: { $error }
clipboard-drop-clear-failed = 清理期间清除剪贴板失败: { $error }
clipboard-notification-cleared = { $cancelled ->
    [true] 剪贴板已清除（已取消）
   *[false] 剪贴板已清除
  }
clipboard-notification-unchanged = { $cancelled ->
    [true] 剪贴板保持不变（您复制了其他内容）（已取消）
   *[false] 剪贴板保持不变（您复制了其他内容）
  }
clipboard-notification-forcibly-cleared = { $cancelled ->
    [true] 剪贴板已清除（无法验证内容）（已取消）
   *[false] 剪贴板已清除（无法验证内容）
  }
clipboard-notification-failed = { $cancelled ->
    [true] 剪贴板清除失败！请手动清除。（已取消）
   *[false] 剪贴板清除失败！请手动清除。
  }

## 提示

prompt-enter-secret = 输入 { $secret_path } 的密钥：
prompt-could-not-read-secret = 无法读取 { $secret_path } 的密钥
error-editor-launch-failed = 无法启动编辑器 '{ $editor }'
error-editor-exited-unsuccessfully = 编辑器 '{ $editor }' 未成功退出，因此密钥保持不变
error-editor-file-not-utf8 = 编辑后的密钥不是有效的 UTF-8
error-editor-file-create-failed = 无法在 '{ $path }' 中为编辑器创建临时文件
editor-file-in-temp-dir = $XDG_RUNTIME_DIR 和 /dev/shm 均不可用，因此编辑器运行期间密钥会写入 '{ $path }' 中的文件。该目录可能位于磁盘上。
editor-file-cleanup-failed = 无法覆盖并删除编辑器临时文件 '{ $path }'：{ $error }
prompt-overwrite-secret = 是否覆盖现有密钥？
prompt-remove-secret = 是否删除现有密钥？
prompt-overwrite-one-time-password = 是否覆盖现有的一次性密码？
prompt-remove-one-time-password = 是否删除现有的一次性密码？

## 错误消息

error-cannot-get-user-confirmation = 无法获取用户确认
error-no-confirmation-from-non-terminal = 无法从非终端输入获取用户确认。使用 --force 跳过确认。
error-could-not-load-configuration = 无法加载配置
error-store-does-not-exist = 名为 '{ $store_name }' 的存储在配置中不存在
error-invalid-line-number = '{ $input }' 不是有效的行号
error-line-number-must-not-be-zero = 行号不能为 0。第一行使用 1,最后一行使用 -1
error-invalid-count = '{ $input }' 不是有效的计数
error-count-must-not-be-zero = 计数不能为 0。使用 1 跳过第一行
error-file-does-not-exist = 文件 '{ $input }' 不存在
error-secret-already-exists = 密钥已存在于 { $secret_path }。使用 --force 覆盖。
error-secret-already-exists-overwrite-or-inplace = 密钥已存在于 { $secret_path }。使用 --force 完全覆盖,或 --inplace 仅修改第一行。
error-password-length-zero = 密码长度不能为 0
error-password-length-too-short = 密码长度对于所需的字符类型来说太短
error-password-no-character-kinds = 必须至少启用一种字符类型
error-password-generation-failed = 无法生成密码：{ $reason }
error-no-secret-found = 在 '{ $secret_path }' 找不到密钥
error-not-allowed-to-remove-secret = 不允许删除 { $secret_path } 的密钥。使用 --force 覆盖。
error-secret-does-not-exist-for-edit = 密钥在 { $secret_path } 不存在。使用 'pasejo secret add' 创建。
error-one-time-password-already-exists = 一次性密码已存在于 { $password_path }。使用 --force 覆盖。
error-not-allowed-to-remove-one-time-password = 不允许删除 { $password_path } 的一次性密码。使用 --force 覆盖。
error-no-one-time-password-found = 在 '{ $password_path }' 找不到一次性密码
error-invalid-one-time-password-secret = 一次性密码的密钥必须是非空的 base32 字符串（字母 A-Z 和数字 2-7）
error-invalid-otpauth-url = otpauth URL 未描述有效的一次性密码
error-hotp-counter-exhausted = HOTP 计数器已达到最大值，无法继续递增
error-no-store-in-configuration = 配置中找不到存储。请先运行 'pasejo store add ...' 添加一个
error-unknown-store = 配置中不存在存储 '{ $store_name }'。运行 'pasejo store list' 查看已配置的存储。
error-decrypt-requires-yes-i-know = 拒绝解密:这会将每个密钥以明文形式打印到 stdout。请传递 --yes-i-know 以确认。
error-no-store-or-global = 配置中找不到存储,且未指定 --global 标志。请先运行 'pasejo store add ...' 添加一个
error-store-name-already-exists = 存储名称已存在。请使用其他名称。
error-store-path-is-directory = 不能将目录用作存储路径。请使用文件路径。
error-cannot-create-store-path = 无法创建存储路径。请检查路径并重试。
error-cannot-get-store-parent = 无法获取存储路径的父目录。请检查路径并重试。
error-cannot-identify-store = 无法识别存储。设置默认存储、使用 --store 指定存储,或使用 --global 全局设置身份。
error-identity-file-already-added = '--file <FILE>' 的值 '{ $file }' 无效：该文件已添加为身份
error-identity-file-not-known = '--file <FILE>' 的值 '{ $file }' 无效：该文件不匹配任何已知身份
error-identity-did-you-mean = 您是指 '{ $stored }' 吗？身份路径按精确匹配进行比较。
error-no-identity-files-to-decrypt = 没有可用于解密的身份文件。请至少添加一个身份以完成存储初始化。
error-invalid-ssh-public-key-format = 无效的 SSH 公钥格式
error-username-empty = 用户名不能为空
error-username-contains-dotdot = 无效的用户名 '{ $username }':不能包含 '..'
error-username-invalid-character = 无效的用户名 '{ $username }':必须以 ASCII 字母或数字开头,并且只能包含 ASCII 字母、数字、'-'、'_' 和 '.'
error-no-public-key-found-in-file = 在 '{ $filename }' 找不到公钥
error-no-public-keys-for-user = { $provider } 用户 '{ $username }' 没有可添加为接收者的公开 SSH 密钥
error-no-public-key-source = 必须为公钥指定至少一个来源
error-no-qrcode-found = 在 '{ $qrcode }' 找不到 QR 码
error-failed-to-decode-qrcode = 无法解码 '{ $qrcode }' 中的 QR 码
error-qr-sandbox-failed = 无法在沙箱中运行 QR 码解码器
error-qr-sandbox-child-signal = QR 码解码器被信号 { $signal } 终止
error-qr-sandbox-not-enforced = 无法应用 QR 码解码器沙箱；运行中的内核不支持 Landlock
error-cannot-determine-store-name = 无法确定存储名称
error-cannot-parse-hook-command = 无法解析命令:{ $command }
error-empty-hook-command = 空的钩子命令:{ $command }
error-failed-to-run-hook = 无法运行钩子 { $command }
error-hook-failed-no-detail = 钩子 { $command } 失败(退出 { $exit })
error-hook-failed-with-detail = 钩子 { $command } 失败(退出 { $exit }):{ $detail }
error-cannot-determine-store-parent-path = 无法确定存储路径 { $path } 的父目录
error-store-path-not-utf8 = 无法替换标记 { $token } 中的 %p:存储路径 { $path } 不是有效的 UTF-8
error-merge-conflict-recipients = 在收件人中检测到合并冲突。请手动解决。
error-merge-conflict = 在 { $value_type }s 中检测到合并冲突。请手动解决。
error-recipient-not-found-in-store = 在存储中找不到收件人
error-cannot-remove-last-recipient = 无法从存储中移除最后一个接收者。请先添加新的接收者，再移除此接收者。
error-cannot-decrypt-common-ancestor-store = 无法解密公共祖先存储
error-cannot-decrypt-current-version-store = 无法解密当前版本存储
error-cannot-decrypt-other-version-store = 无法解密其他版本存储
error-cannot-read-file = 无法读取 '{ $path }' 的文件
error-downloading-public-key-failed = 从 { $provider } 下载公钥失败
error-download-user-not-found = { $provider } 上没有用户 '{ $username }'
error-download-rate-limited = { $provider } 限制了请求频率；请稍候再试
error-download-server-error = { $provider } 返回了错误 (HTTP { $status })
error-download-timeout = 从 { $provider } 下载公钥超时；请检查网络连接或增大 key-download-timeout-seconds
error-cannot-determine-parent-directory = 无法确定 { $path } 的父目录
error-failed-to-create-directory = 无法创建目录 { $path }
error-failed-to-write-file = 无法写入 { $path }
error-failed-to-fsync-file = 在 { $path } 上 fsync 失败
error-failed-to-rename-file = 无法将 { $from } 重命名为 { $to }
error-failed-to-create-file = 无法创建 { $path }
error-could-not-resolve-config-path = 无法解析配置的绝对路径
error-could-not-determine-config-path = 无法确定配置路径
error-could-not-resolve-store-path = 无法解析存储的绝对路径
error-could-not-create-config-dir = 无法创建配置目录
error-could-not-migrate-legacy-config = 无法迁移旧版配置文件
error-could-not-move-file = 无法移动文件
error-could-not-copy-file = 无法复制文件
error-could-not-remove-source-after-copy = 复制后无法删除源文件
error-could-not-read-configuration = 无法读取配置
error-could-not-serialize-migrated-config = 无法序列化迁移后的配置
error-could-not-store-configuration = 无法保存配置
error-could-not-load-migrated-config = 无法加载迁移后的配置
error-could-not-serialize-configuration = 无法序列化配置
error-config-not-valid-toml = { $path } 的配置文件不是有效的 TOML
error-config-invalid-integer = 选项 '{ $option }' 的值 '{ $value }' 无效：应为非负整数
error-config-invalid-boolean = 选项 '{ $option }' 的值 '{ $value }' 无效：应为 true、false、1、0、yes、no、y、n 之一
error-could-not-open-store-for-lock = 无法打开存储文件进行锁定:{ $path }
error-could-not-acquire-store-lock = 无法获取存储文件的锁:{ $path }
error-store-lock-wait-timed-out = 等待存储文件的锁超时:{ $path }
error-cannot-encrypt-store = 无法加密存储
error-failed-to-run-command = 无法运行命令 { $binary }
error-command-exited-with = 命令 { $binary } 以 { $exit } 退出
error-browser-host-requires-pipe = 浏览器主机需要来自连接的浏览器扩展的管道标准输入。请通过浏览器的原生消息传递清单运行此命令，而不是直接从终端运行。
error-browser-store-file-missing = 存储文件不存在:{ $path }
error-browser-identity-passphrase-protected = 浏览器主机无法请求加密身份的密码短语。请为此存储使用未加密或基于插件的身份。
prompt-browser-host-pinentry = PIN 或密码短语
error-browser-message-too-large = { $length } 字节的消息超出了 { $limit } 字节的原生消息限制
error-browser-response-too-large = { $length } 字节的响应超出了 { $limit } 字节的原生消息限制
error-browser-install-manifest-no-browser = 请至少指定一个 --browser 以安装其原生消息传递清单。
error-browser-install-manifest-needs-extension-id = Chromium 系浏览器的扩展 ID 由解压扩展的公钥派生，因此浏览器 '{ $browser }' 需要 --extension-id。
error-browser-install-manifest-extension-id-multiple-browsers = --extension-id 不能与多个 --browser 一起使用，因为每个浏览器都需要各自的扩展 ID。请为每个浏览器分别运行一次命令。
error-browser-install-manifest-unsupported-os = '{ $os }' 不支持安装原生消息传递清单。
error-browser-install-manifest-registry-failed = 无法写入本地消息注册表项 '{ $key }'
error-browser-install-manifest-no-home-dir = 无法确定用户的主目录。
error-browser-install-manifest-no-data-dir = 无法确定用户的数据目录。
error-browser-install-manifest-no-current-exe = 无法确定正在运行的 pasejo 二进制文件的路径。
browser-install-manifest-written = 已将 '{ $browser }' 的原生消息传递清单写入 '{ $path }'
browser-install-manifest-print-wrapper-note = 注意：打印的清单引用了位于 '{ $path }' 的包装脚本，但此命令并未安装它。请改用 --binary 引用现有的可执行文件。
error-browser-install-manifest-invalid-env = --env 条目 '{ $entry }' 不是 KEY=VALUE 形式。
error-browser-install-manifest-invalid-env-name = --env 名称 '{ $name }' 不是有效的 POSIX 环境变量名（必须以字母或下划线开头，且仅包含字母、数字和下划线）。
error-browser-install-manifest-invalid-env-value-windows = 无法将 '{ $name }' 的 --env 值写入 Windows 包装脚本：它包含双引号或换行符。
error-browser-install-manifest-invalid-host-name = --host-name '{ $name }' 不是 { $browser } 的有效原生消息主机名：它必须由以点分隔的字母、数字和下划线部分组成，除 firefox 外的所有浏览器都必须使用小写。
error-browser-install-manifest-invalid-extension-id = --extension-id '{ $id }' 不是有效的扩展 ID：只能包含 ASCII 字母、数字、'@'、'.'、'_'、'-' 和花括号。
error-browser-release-log-lock = 无法锁定浏览器发放日志：{ $path }
error-browser-release-log-no-data-dir = 无法确定浏览器发放日志的数据目录。
error-browser-release-log-failed = 无法在浏览器发放日志中记录本次发放，因此未发送凭据。
error-browser-caller-not-allowed = 浏览器扩展 '{ $caller }' 不允许使用此 pasejo 主机。请为该扩展运行 `pasejo browser install-manifest`，或使用 --allowed-caller 添加它。
error-browser-caller-unnamed = 浏览器未指明启动此 pasejo 主机的扩展，因此无法与 --allowed-caller 进行核对。
browser-log-empty = 尚未向浏览器发放任何凭据。
browser-log-unreadable-lines = 已跳过浏览器发放日志中 { $count } 行无法读取的内容。
browser-log-header-time = 时间
browser-log-header-domain = 域名
browser-log-header-store = 存储
browser-log-header-secret = 密钥
browser-log-header-caller = 调用方
