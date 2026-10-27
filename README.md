# 截图吧插件

这里开发和打包截图吧的独立插件。当前包含 JSON 格式化预览、Markdown 文件预览、RTF 富文本预览、格式检查器、JSON 格式化复制操作及敏感内容采集过滤器。

在 Windows 安装 Rust MSVC 工具链与 Python 3 后，运行：

```powershell
python scripts/package.py
```

生成的 `.jtbplugin` 文件位于 `dist/`。在截图吧剪贴板窗口的设置菜单选择“安装插件”，选择对应文件；安装后打开条目详情预览即可使用。插件是本机可执行程序，只有信任的插件才应该安装。

## 插件协议 v1

包是 ZIP 文件，根目录包含 `plugin.json` 和清单中指定的 Windows 可执行文件。清单包含 `id`、`name`、`version`、`api_version: 1`、`type: "preview"`、`content_types`（`text`、`image` 或 `file`）和相对 `command`。插件通过标准输入接收一个 UTF-8 JSON 对象，字段包括 `api_version`、`type`、`content_type`、`sub_kind`、`content`、可选 `html` 和 `paths`；图片大小不超过 4 MiB 时包含 `image_base64`。

标准输出返回一个 UTF-8 JSON 对象：`{"api_version":1,"kind":"text|html|markdown|image","content":"..."}`；图片使用 Base64 编码。无法处理时返回 `{"api_version":1,"supported":false}`。错误、超时、无效响应都会回退到截图吧原有预览。插件应尽快退出，且不得往标准输出写日志。HTML 与 Markdown 由截图吧渲染，不加载外部资源。

清单可选 `formats` 字符串数组，声明需要的原始剪贴板格式名称（最多 8 项）；宿主将已保存且不大于 4 MiB 的对应数据通过 `formats_base64` 对象提供。RTF 插件请求 `Rich Text Format`，把文字、粗体、斜体、下划线及颜色转换为 HTML；复杂的嵌入对象与部分排版可能不能还原。超大或无效的 RTF 会回退到原有预览。

插件按可选整数 `priority` 从高到低尝试，相同优先级按目录名称排序；没有插件能处理时使用内置预览。RTF 插件在条目同时具有 HTML 时让宿主继续处理。安装后可在截图吧“管理插件”中启用、停用或卸载。

格式检查器声明 `modes: ["formats"]` 和 `formats: ["*"]`。条目右键的“格式”窗口向插件传入 `format_names`；用户选择一个格式后，`selected_format` 指明它，`formats_base64` 至多提供选中格式的数据。选择“全部”时最多提供 8 种格式、合计 4 MiB 的原始数据。检查器显示每种数据的前 256 字节；不在限额内的数据仍列出名称。

## 操作插件

清单中使用 `type: "action"`。用户从条目右键的“插件”菜单主动运行，宿主通过标准输入传入 `type: "action"` 及条目内容；可选 `formats` 按名称申请原始数据。插件输出 `{"api_version":1,"kind":"text","content":"..."}`，宿主将结果复制到剪贴板；失败和超时不会更改剪贴板。`name_i18n` 可提供各语言菜单名称。插件可以用任意语言开发，只要包内含 Windows `.exe` 并遵守此协议。

“运行文本脚本并复制”插件可在插件设置里编辑 [Rhai](https://rhai.rs/) 脚本，提供 `content` 和 `source_app` 变量；脚本最后需返回字符串。例如 `let text = content; text.trim(); text.to_upper()` 会去除首尾空白并转为大写。脚本有运算和输出大小限制，未开放文件或网络模块；它不是 CopyQ 的 JavaScript API。可以用不同的插件 ID 打包多个独立脚本。

## 采集过滤插件

清单使用 `type: "capture_filter"`。宿主在保存记录和图片文件之前，将 `content_type`、文本 `content`、`source_app` 和文件 `paths` 发送给匹配的插件；返回 `{"api_version":1,"allow":false}` 可阻止入史。没有匹配插件时正常保存。已启用的过滤插件出错、超时或收到超过 8 MiB 的请求时，匹配的内容不会保存，宿主会提示用户修复或停用插件。安装、更新和停用与采集互斥，避免更新期间漏过滤。

敏感内容过滤器识别私钥标记、AWS 访问密钥、常见服务令牌和 JWT。它是启发式保护，不保证识别所有秘密；样例或文档中的相似字符串也可能被跳过。可以在“管理插件”中停用。截图吧仍独立尊重来源应用标记的私密剪贴板内容。

“忽略指定应用”插件可在“管理插件 → 插件设置”中填写程序名，每行一个，例如 `password-manager.exe`。按完整程序文件名不区分大小写匹配；系统无法确定来源时不会匹配。配置单独保存，更新或卸载插件不会删除配置。清单中的 `settings` 数组目前支持 `{"key":"...","type":"multiline","label":"..."}`；插件通过环境变量 `JIETUBA_PLUGIN_CONFIG` 获取其 JSON 配置路径。
