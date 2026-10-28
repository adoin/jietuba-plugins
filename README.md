# 截图吧插件

这里开发和打包截图吧的独立扩展。所有插件使用统一的程序级扩展协议 v2；旧协议包不能安装到此版本的截图吧。

在 Windows 安装 Rust MSVC 工具链与 Python 3 后运行：

```powershell
python scripts/package.py
```

生成的 `.jtbplugin` 文件位于 `dist/`。在截图吧系统托盘右键菜单或剪贴板窗口的设置菜单选择“安装插件”。插件以当前用户权限运行，只应安装信任的来源。

## 能力和串联

统一清单使用 `api_version: 2`、`type: "extension"` 和 `contributes` 声明命令、事件钩子、工具栏栏位及剪贴板能力。一个插件可以组合多种能力。`pipeline` 按顺序调用不同插件的命令，上一命令的图片或文本输出成为下一命令的输入。

同时安装 `screenshot-image-info` 和 `screenshot-dimensions` 后，在截图工具栏的“…”中选择“复制截图尺寸”，可以运行图片 → 尺寸文本 → 格式化文本的跨插件处理链。`script-transform` 的文本命令也可以处理其他插件产生的文本。

截图事件包括 `screenshot.started`、`screenshot.selection_confirmed`、`screenshot.before_export`、`screenshot.exported`、`screenshot.cancelled` 和 `screenshot.ended`。剪贴板采集过滤器声明 `clipboard.before_capture`；预览、动作、条目元数据和备份分别在 `contributes.clipboard` 下声明。完整的清单格式、数据类型、生命周期边界和大小限制见截图吧主项目的 `docs/plugins.md`。

## 已实现的插件

- JSON、Markdown、RTF 预览和原始格式检查。
- JSON 格式化及可配置 Rhai 文本脚本。
- 敏感内容过滤和指定应用排除。
- 条目备注与标签。
- 剪贴板加密备份与恢复，使用 age 密码加密。
- 截图尺寸读取和格式化示例。

采集过滤是启发式保护，可能漏掉秘密或跳过相似的示例文本。来源应用排除依赖系统能识别来源进程。加密备份密码不会保存，丢失后无法恢复备份。
