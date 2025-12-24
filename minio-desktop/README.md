# MinIO Tauri 桌面客户端

一个基于 Tauri 构建的 MinIO 客户端，支持文件上传、下载、预览等功能。

## 功能特性

- 🔐 **SSE-C 加密支持** - 支持客户端加密/解密
- 📁 **文件管理** - 浏览、上传、下载、删除 MinIO 存储桶中的文件
- 🖼️ **媒体预览** - 支持图片和视频预览
- 📹 **视频播放** - 支持大视频流式播放
- ⚡ **多线程上传** - 支持大文件分片上传
- 🔄 **断点续传** - 支持上传断点续传
- 🎨 **现代化界面** - 使用 Vue 3 + Tauri 构建

## 系统要求

- **操作系统**: macOS 10.15 或更高版本
- **Node.js**: 18.x 或更高版本
- **Rust**: 最新稳定版
- **Tauri**: 1.x

## 开发环境搭建

### 1. 安装依赖

```bash
# 安装 Node.js 依赖
npm install

# 安装 Tauri CLI
cargo install tauri-cli

# 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 启动开发服务器

```bash
# 启动前端开发服务器
npm run dev

# 在另一个终端启动 Tauri 开发模式
npm run tauri dev
```

## 打包成 macOS 客户端

### 1. 前提条件

确保已安装以下工具：

```bash
# 检查 Rust 安装
rustc --version

# 检查 Node.js 安装
node --version

# 检查 Cargo 安装
cargo --version

# 安装 Tauri CLI（如果尚未安装）
cargo install tauri-cli
```

### 2. 构建 macOS 应用

#### 方法一：使用命令行构建

```bash
# 构建 macOS 应用（默认为调试版本）
npm run tauri build

# 或者直接使用 Cargo
cargo tauri build
```

#### 方法二：构建发布版本

```bash
# 构建发布版本
npm run tauri build -- --release

# 或者
cargo tauri build --release
```

### 3. 构建产物说明

构建完成后，产物将位于以下目录：

```
src-tauri/target/
├── debug/          # 调试版本产物
│   └── bundle/macos/
│       └── minio-desktop_x.x.x_x64.dmg
├── release/        # 发布版本产物
│   └── bundle/macos/
│       └── minio-desktop_x.x.x_x64.dmg
```

- `.app` - macOS 应用程序包
- `.dmg` - macOS 磁盘映像安装包

### 4. 自定义构建配置

构建配置位于 `src-tauri/tauri.conf.json` 文件中：

```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "tauri": {
    "bundle": {
      "active": true,
      "targets": "all",
      "identifier": "com.minio.desktop",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "publisher": "MinIO Team",
      "category": "DeveloperTool",
      "shortDescription": "MinIO Desktop Client",
      "longDescription": "A desktop client for MinIO object storage with SSE-C encryption support."
    }
  }
}
```

### 5. 签名和公证（可选）

如果需要在 macOS 上分发应用，建议进行代码签名和公证：

```bash
# 安装 Apple Developer 证书
# 1. 从 Apple Developer Portal 下载证书
# 2. 导入到 Keychain Access

# 构建并签名
npm run tauri build -- --release --features signing
```

在 `tauri.conf.json` 中配置签名：

```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "signingIdentity": "Developer ID Application: Your Name (XXXXXXXXXX)",
        "entitlements": "entitlements.plist"
      }
    }
  }
}
```

### 6. 常见问题

#### 构建失败

1. **Rust 版本问题**：
   ```bash
   rustup update stable
   rustup default stable
   ```

2. **依赖下载慢**：
   ```bash
   # 配置 Rust 国内镜像
   echo '[source.crates-io]' > ~/.cargo/config
   echo 'replace-with = "rsproxy"' >> ~/.cargo/config
   echo '[source.rsproxy]' >> ~/.cargo/config
   echo 'registry = "https://rsproxy.cn/crates.io-index"' >> ~/.cargo/config
   ```

3. **权限问题**：
   ```bash
   # 确保有足够的磁盘空间
   # 确保对项目目录有写权限
   ```

#### 应用运行问题

1. **"应用已损坏，无法打开"**：
   - 这是 macOS Gatekeeper 的安全限制
   - 临时解决方案：在终端运行 `xattr -d com.apple.quarantine /path/to/app`

2. **SSL 证书问题**：
   - 应用支持自定义 SSL 证书验证
   - 可配置跳过证书验证（仅开发环境）

## 使用说明

### 连接 MinIO 服务器

1. 启动应用
2. 点击"连接"按钮
3. 输入 MinIO 服务器信息：
   - 服务器地址（如：https://localhost:9000）
   - 访问密钥 ID
   - 机密访问密钥
   - 存储桶名称

### 上传文件

1. 选择要上传的文件
2. 选择目标存储桶
3. 可选择是否启用 SSE-C 加密
4. 点击上传

### 预览媒体文件

1. 在文件列表中选择图片或视频
2. 点击预览按钮
3. 支持全屏播放
4. 使用方向键切换文件

## 安全说明

- **SSE-C 加密**：支持客户端加密，密钥由用户管理
- **证书验证**：支持自定义 SSL 证书验证
- **本地存储**：敏感信息加密存储在本地数据库

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

[MIT License](LICENSE)