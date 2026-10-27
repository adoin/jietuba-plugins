# 截图吧插件

这里开发和打包截图吧的独立插件。当前包含 JSON 格式化预览和 Markdown 文件预览。

在 Windows 安装 Rust MSVC 工具链与 Python 3 后，运行：

```powershell
python scripts/package.py
```

生成的 `.jtbplugin` 文件位于 `dist/`。在截图吧剪贴板窗口的设置菜单选择“安装插件”，选择对应文件；安装后打开条目详情预览即可使用。插件是本机可执行程序，只有信任的插件才应该安装。

## 插件协议 v1

包是 ZIP 文件，根目录包含 `plugin.json` 和清单中指定的 Windows 可执行文件。清单包含 `id`、`name`、`version`、`api_version: 1`、`type: "preview"`、`content_types`（`text`、`image` 或 `file`）和相对 `command`。插件通过标准输入接收一个 UTF-8 JSON 对象，字段包括 `api_version`、`type`、`content_type`、`sub_kind`、`content`、可选 `html` 和 `paths`；图片大小不超过 4 MiB 时包含 `image_base64`。

标准输出返回一个 UTF-8 JSON 对象：`{"api_version":1,"kind":"text|html|markdown|image","content":"..."}`；图片使用 Base64 编码。无法处理时返回 `{"api_version":1,"supported":false}`。错误、超时、无效响应都会回退到截图吧原有预览。插件应尽快退出，且不得往标准输出写日志。HTML 与 Markdown 由截图吧渲染，不加载外部资源。

插件按目录名称排序，适用的外部预览插件优先；没有插件能处理时使用内置预览。宿主目前不会向插件提供 RTF 或其他原始剪贴板格式。
