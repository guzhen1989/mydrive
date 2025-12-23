```markdown
# MinIO Tauri Desktop (示例)

功能
- 连接 MinIO (endpoint, port, accessKey, secretKey, useSSL)
- 支持 SSE-C（用户提供加密 key）
- 列出 buckets / 列出对象
- 图片预览（点击图片打开预览）
- 视频在线播放（HTML5 video，基于本地临时文件渐进式写入）
- 断点续传（上传使用 S3 Multipart Upload；前端可分块上传并在中断后继续）
- 下载支持 Range 恢复（可从指定偏移续传）
- macOS 优化（可用 `tauri build` 打包）

注意
- 本模板以演示为主：在生产环境请加强 secret 存储（使用 OS keychain、更严格的权限、代码审计）。
- 依赖 Rust 和 Node 工具链：请安装 Rust (stable) 与 Node.js (建议 >=18)，还需要安装 Tauri prerequisites（请参见 Tauri 文档）。
- AWS Rust SDK 与 MinIO 兼容，使用 S3 API 与 MinIO 通信。

快速运行（开发）
1. 安装前端依赖：
   cd frontend
   npm install

2. 启动开发：
   npm run tauri dev
   （第一次可能会构建 Rust 后端）

打包（macOS）
- 在 macOS 上运行：
  cd frontend
  npm run tauri build

项目结构（重要文件）
- frontend/           -- 前端 (React + Vite)
  - src/
    - App.tsx
    - components/ BucketList, ObjectList, Viewer
    - vite entry...
  - package.json
- src-tauri/          -- Tauri / Rust 后端
  - src/main.rs       -- 启动与命令绑定
  - src/s3.rs         -- S3 操作（list, multipart, download, presign）
  - Cargo.toml
  - tauri.conf.json

安全提示
- SSE-C 需要用户提供 base64 的 key（SSE-C 要求 256-bit key 的 base64 表示）；请在 UI 上提示用户如何生成 / 存储。
- 我在示例中使用本地文件（临时）作为视频播放介质；如果对隐私/安全有要求，请实现更严格的存储策略或清理机制。
```