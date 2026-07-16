[English](README.md) · [Русский](README.ru.md) · [Беларуская](README.be.md) · [Қазақша](README.kk.md) · **简体中文**

# Ninjutso Battery Tray

[![最新版本](https://img.shields.io/github/v/release/DrStopor/ninjutso-battery-tray?display_name=tag&sort=semver&label=release)](https://github.com/DrStopor/ninjutso-battery-tray/releases/latest)
![Windows](https://img.shields.io/badge/Windows-tray-0078D4?logo=windows11)
[![Rust 2024](https://img.shields.io/badge/Rust-2024-000000?logo=rust)](Cargo.toml)
[![安装程序](https://img.shields.io/badge/installer-Inno%20Setup-2ea44f)](installer/ninjutso-battery-tray.iss)
[![许可证](https://img.shields.io/badge/license-source--available-f59e0b)](LICENSE)

一款专注、轻量的 Windows 托盘工具，无需打开完整的配套软件，即可随时查看 Ninjutso Ten 的电量。

![Ninjutso Battery Tray 简体中文状态浮窗：Ninjutso Ten 通过 USB 连接并正在充电，电量为 100%](docs/assets/screenshots/flyout.zh-CN.png)

## 功能亮点

- **托盘内直接显示精确电量。** 高对比度数字在通知区域的实际图标尺寸下依然清晰。
- **同时支持接收器和有线 USB。** 无论通过无线接收器连接，还是使用 USB 为鼠标充电，都能读取电量。
- **简洁紧凑的状态卡片。** 左键单击即可查看电量百分比、连接方式、充电状态和连接状态。
- **自动与手动刷新。** 电量每分钟自动更新，也可从托盘菜单立即刷新。
- **本地且注重隐私。** 无云服务、无需账户、不访问网络、无遥测。
- **单实例保护。** 再次启动应用不会产生重复的托盘图标。
- **五种界面语言。** 英语、俄语、白俄罗斯语、哈萨克语和简体中文。

## 支持的设备

Ninjutso Battery Tray 目前支持 **Ninjutso Ten** 的以下连接方式：

- 无线接收器；
- 通过 USB 有线连接，包括鼠标充电时。

如果未检测到鼠标，请通过 USB 连接鼠标或将其开启，然后在托盘菜单中选择 **刷新**。

## 安装

[**从 GitHub 下载最新版本。**](https://github.com/DrStopor/ninjutso-battery-tray/releases/latest)

- 建议大多数用户下载 `NinjutsoBatteryTraySetup-*.exe`，它支持开机自启和 Windows 标准卸载。
- 如需免安装使用，请下载 `ninjutso-battery-tray-*-portable.zip`。
- `SHA256SUMS.txt` 提供用于校验下载文件的哈希值。

默认仅为当前用户安装，无需管理员权限。安装向导还可启用开机自启，或切换为所有用户安装。向导提供英语、俄语、白俄罗斯语、哈萨克语和简体中文。

启动后，应用只会出现在系统托盘中。Windows 可能将新图标收进“隐藏的图标”区域；如需始终显示，请将图标拖到通知区域的可见区域。

## 使用方法

左键单击托盘图标可打开状态卡片。再次单击图标、单击卡片外部或按下 `Esc` 均可关闭卡片；单击卡片内部不会将其关闭。卡片保持显示时会自动刷新，无需重新打开即可呈现 USB、接收器和鼠标断开连接之间的变化。

右键单击图标可打开菜单：

- **刷新** — 重新读取电量；
- **显示电量百分比** — 在数字图标和图形图标之间切换；
- **语言** — 更改界面语言；
- **退出** — 关闭应用。

在数字模式下，边框颜色表示电量区间：绿色为 50–100%，琥珀色为 20–49%，红色为 0–19%。准确的充电和连接状态始终可在状态卡片和工具提示中查看。

所选语言和图标模式会自动保存。

## 语言

界面提供英语、俄语、白俄罗斯语、哈萨克语和简体中文（`zh-CN`）。首次启动时，应用会优先采用受支持的 Windows 显示语言；如果不受支持，则改用英语。

`zh-CN` 使用面向中国大陆的简体中文。

## 管理安装

### 开机自启

再次运行安装程序，即可启用或关闭 **随 Windows 启动**。

### 更新

下载新的安装程序并直接覆盖现有版本进行安装。安装模式和应用设置都会保留。

### 卸载

打开 **Windows 设置 → 应用 → 已安装的应用**，找到 **Ninjutso Battery Tray**，然后选择 **卸载**。

## 隐私

电量数据仅通过 Windows HID API 在本机读取。应用不会向任何位置发送数据，也不包含分析、广告或后台更新检查功能。

## 支持

[在 GitHub 提交问题](https://github.com/DrStopor/ninjutso-battery-tray/issues)时，请附上 Windows 版本、连接方式（接收器或 USB）和应用版本。

## 许可证

可根据 [LICENSE](LICENSE) 中的条款使用、复制、修改和再分发源代码。Commons Clause 禁止出售本工具本身，或出售其主要价值实质上来自本工具的产品或服务。因此本项目属于 source-available，而非 OSI 定义下的开源软件。

<details>
<summary><strong>从源代码构建</strong></summary>

环境要求：

- Windows；
- 带 Cargo 的 Rust 工具链；
- 带 `rc.exe` 的 Windows SDK，用于将图标嵌入可执行文件；
- Inno Setup 6，用于构建安装程序。

```powershell
cargo test
cargo build --release
```

构建全部发布文件：

```powershell
.\tools\package-release.ps1
```

可执行文件将生成在 `target\release\ninjutso-battery-tray.exe`，打包文件将生成在 `dist`。面向开发者的 HID 协议说明位于 [docs/hid-protocol.md](docs/hid-protocol.md)，版本变更记录见 [CHANGELOG.md](CHANGELOG.md)。

</details>
