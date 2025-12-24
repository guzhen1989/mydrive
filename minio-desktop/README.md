# MinIO Tauri Desktop Client

基于 Tauri + Vue 3 的 MinIO 桌面客户端应用。

## 功能特性

### 已实现
- ✅ 连接管理 (endpoint, port, accessKey, secretKey, useSSL)
- ✅ 存储桶列表与管理 (创建、删除)
- ✅ 对象浏览与操作 (列表、删除、文件夹导航)
- ✅ 数据库持久化 (连接配置、传输任务)
- ✅ 基础 UI 组件 (连接配置、存储桶列表、对象浏览器、传输面板)

### 待完善
- ⏳ SSE-C 加密支持
- ⏳ 断点续传 (Multipart Upload)
- ⏳ 图片预览
- ⏳ 视频流式播放
- ⏳ 文件上传下载实现
- ⏳ 传输进度实时更新

## 技术栈

### 后端 (Rust)
- Tauri 1.5
- AWS SDK for Rust (S3 客户端)
- Tokio (异步运行时)
- Rusqlite (SQLite 数据库)
- Axum (HTTP 服务器,用于视频流代理)

### 前端 (Vue)
- Vue 3
- TypeScript
- Pinia (状态管理)
- Vue Router
- Vite

## 项目结构

```
minio-desktop/
├── src/                    # 前端源码
│   ├── api/               # API 调用层
│   ├── components/        # Vue 组件
│   ├── stores/            # Pinia stores
│   ├── views/             # 页面视图
│   ├── App.vue
│   ├── main.ts
│   └── style.css
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── commands/      # Tauri 命令
│   │   ├── db.rs          # 数据库操作
│   │   ├── minio.rs       # MinIO 客户端
│   │   ├── models.rs      # 数据模型
│   │   ├── streaming.rs   # 视频流服务器
│   │   ├── error.rs       # 错误处理
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## 开发指南

### 环境要求
- Node.js 16+
- Rust 1.70+
- 支持 macOS (其他平台未测试)

### 安装依赖

```bash
# 安装前端依赖
npm install
```

### 开发模式

```bash
# 启动开发服务器
npm run tauri dev
```

### 构建应用

```bash
# 构建生产版本
npm run tauri build
```

## 数据库结构

应用使用 SQLite 存储本地数据,数据库位置:
- macOS: `~/Library/Application Support/minio-desktop/data.db`

### 表结构

**connection_config** - 连接配置
- id, endpoint, port, access_key, secret_key, use_ssl, last_connected

**transfer_tasks** - 传输任务
- task_id, task_type, file_name, local_path, bucket_name, object_key, file_size, upload_id, part_size, total_parts, completed_parts, transferred_bytes, status, error_message, use_encryption, created_at, updated_at, completed_at

**encryption_keys** - 加密密钥
- key_id, key_value, key_md5, enabled, created_at

## 使用说明

### 1. 连接到 MinIO

首次启动应用时,需要配置 MinIO 连接:
- 服务器地址 (如: localhost)
- 端口 (默认: 9000)
- Access Key
- Secret Key  
- 是否使用 SSL

点击"测试连接"验证配置,然后点击"连接"保存。

### 2. 管理存储桶

- 查看所有存储桶
- 点击"+"按钮创建新存储桶
- 点击存储桶名称进入对象列表

### 3. 浏览对象

- 查看存储桶内的文件和文件夹
- 双击文件夹进入子目录
- 使用面包屑导航返回上级目录
- 点击"删除"按钮删除对象

### 4. 传输任务

传输任务面板显示所有上传和下载任务:
- 查看任务进度
- 暂停/继续任务
- 取消任务

## 后续开发计划

### 阶段 1: 完善传输功能
- 实现完整的文件上传下载逻辑
- 实现 Multipart Upload 分块上传
- 实现断点续传机制
- 实时进度更新和事件通知

### 阶段 2: 媒体预览
- 图片预览窗口
- 视频流式播放 (基于 HTTP Range 请求)

### 阶段 3: 加密支持
- SSE-C 密钥管理
- 加密上传下载

### 阶段 4: 优化和测试
- 性能优化
- 错误处理完善
- macOS 平台优化
- 单元测试和集成测试

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request!
