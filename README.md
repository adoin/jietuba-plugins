# 截图妈插件

这里开发和打包截图妈的独立扩展。所有插件使用统一的程序级扩展协议 v2；旧协议包不能安装到此版本的截图妈。

在 Windows 安装 Rust MSVC 工具链与 Python 3 后运行：

```powershell
python scripts/package.py
```

生成的 `.jtbplugin` 文件位于 `dist/`。在截图妈系统托盘右键菜单或剪贴板窗口的设置菜单选择“安装插件”。插件以当前用户权限运行，只应安装信任的来源。

推送分支或创建 Pull Request 只运行检查，不发布。发布时先把 `Cargo.toml` 和每个插件清单的版本更新为同一版本并提交，再对该提交推送 `v<版本>` 标签；例如 `git tag v0.5.0`、`git push origin v0.5.0`。带后缀的标签（如 `v0.5.0-beta.1`）生成预发布。标签可指向包含发布工作流的任意分支提交；标签不匹配版本时不会发布。

## 能力和串联

统一清单使用 `api_version: 2`、`type: "extension"` 和 `contributes` 声明命令、事件钩子、工具栏栏位及剪贴板能力。一个插件可以组合多种能力。`pipeline` 按顺序调用不同插件的命令，上一命令的图片或文本输出成为下一命令的输入。

截图事件包括 `screenshot.started`、`screenshot.selection_confirmed`、`screenshot.before_export`、`screenshot.exported`、`screenshot.cancelled` 和 `screenshot.ended`。剪贴板采集过滤器声明 `clipboard.before_capture`；预览、动作、条目元数据和备份分别在 `contributes.clipboard` 下声明。完整的清单格式、数据类型、生命周期边界和大小限制见截图妈主项目的 `docs/plugins.md`。

## 已实现的插件

- JSON、Markdown、RTF 预览和原始格式检查。
- JSON 格式化及可配置 Rhai 文本脚本。
- 敏感内容过滤和指定应用排除。
- 条目备注与标签。
- 剪贴板加密备份与恢复，使用 age 密码加密。

采集过滤是启发式保护，可能漏掉秘密或跳过相似的示例文本。来源应用排除依赖系统能识别来源进程。加密备份密码不会保存，丢失后无法恢复备份。
