# SPDX-FileCopyrightText: The pasejo Authors
# SPDX-License-Identifier: 0BSD

## 接收者

recipient-added = 已添加接收者 '{ $public_key }'
recipient-removed = 已移除接收者 '{ $public_key }'
recipient-does-not-exist-ignored = 接收者 '{ $public_key }' 在存储中不存在 - 忽略

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

## 钩子执行

execute-pull-hooks = 正在执行存储 '{ $store_name }' 的拉取钩子
execute-push-hooks = 正在执行存储 '{ $store_name }' 的推送钩子

## 合并冲突

merge-conflict-recipient-names = 公钥 '{ $public_key }' 的接收者合并冲突：名称 '{ $first_name }' 与 '{ $second_name }' 不同
merge-conflict-recipient-removed-and-renamed = 公钥 '{ $public_key }' 的接收者合并冲突：一个版本中已移除，另一个版本中重命名为 '{ $new_name }'
merge-conflict-values = '{ $secret_path }' 的 { $value_type } 合并冲突：两个版本中的值不同
merge-conflict-removed-and-modified = '{ $secret_path }' 的 { $value_type } 合并冲突：一个版本中已移除，另一个版本中已修改

## 列表输出 (stdout)

list-global-identity = 全局: { $identity_file }
list-store-identity = 存储: { $identity_file }
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
