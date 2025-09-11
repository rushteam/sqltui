# SQLView - MySQL TUI Client

一个使用 Rust 和 ratatui 构建的现代化 MySQL 终端客户端。

## 功能特性

- 🗄️ **数据库浏览** - 查看数据库和表列表
- 📋 **表结构查看** - 显示详细的表结构信息
- 🔍 **SQL 查询** - 支持执行 SQL 查询
- ⌨️ **键盘导航** - 流畅的键盘操作体验
- 🎨 **现代化界面** - 基于 ratatui 的美观 TUI

## 安装要求

- Rust 1.70+
- MySQL 服务器

## 快速开始

### 1. 克隆并构建

```bash
git clone <repository-url>
cd sqlview
cargo build --release
```

### 2. 运行程序

```bash
# 使用默认参数（localhost:3306, root用户）
cargo run

# 指定连接参数
cargo run -- -h localhost -u root -p your_password

# 连接到指定数据库
cargo run -- -h localhost -u root -p root123 -d testdb
```

### 3. 命令行参数

```
-h, --host <HOST>        MySQL 主机地址 (默认: localhost)
-P, --port <PORT>        MySQL 端口 (默认: 3306)
-u, --username <USER>    用户名 (默认: root)
-p, --password <PASS>    密码 (默认: 空)
-d, --database <DB>      指定数据库 (可选)
```

## 使用说明

### 界面布局

```
┌─────────────────────────────────────────────────────────┐
│ [MYSQL_CLIENT] READY | DB: testdb | MySQL: 8.0.33     │ ← 状态栏
├─────────────────┬───────────────────────────────────────┤
│ 数据库列表      │ 主内容区域                            │
│ • testdb        │ 显示数据库/表信息、表结构、查询结果   │
│ • mysql         │                                     │
│ • sys           │                                     │
│                 │                                     │
│ 选中: testdb    │                                     │
│ Up/Down 移动    │                                     │
├─────────────────┴───────────────────────────────────────┤
│ [CMD_MODE] > mysql>                                    │ ← 输入栏
└─────────────────────────────────────────────────────────┘
```

### 快捷键

| 按键 | 功能 |
|------|------|
| `↑/↓` | 上下导航 |
| `Enter` | 选择/确认 |
| `Esc` | 返回上一级 |
| `d` | 查看数据库详情 |
| `t` | 查看表详情 |
| `s` | 切换数据库 |
| `\` | 进入 SQL 模式 |
| `q` | 退出程序 |

### 操作流程

1. **选择数据库** - 使用上下键选择数据库，按 Enter 进入
2. **查看表列表** - 在数据库中选择表，按 Enter 查看表结构
3. **执行 SQL** - 按 `\` 进入 SQL 模式，输入查询语句
4. **返回导航** - 按 `Esc` 返回上一级，按 `s` 返回数据库列表

## 开发

### 项目结构

```
src/
├── main.rs          # 程序入口
├── config/          # 配置管理
├── db/             # 数据库连接和查询
├── models/         # 数据模型
└── ui/             # TUI 界面组件
    ├── app.rs      # 主应用逻辑
    └── components/ # UI 组件
```

### 构建命令

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy
```

## 技术栈

- **Rust** - 系统编程语言
- **ratatui** - 终端用户界面库
- **sqlx** - 异步 SQL 工具包
- **crossterm** - 跨平台终端操作
- **clap** - 命令行参数解析

## 许可证

MIT License
