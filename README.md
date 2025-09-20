# sqltui-rs - MySQL TUI Client

一个使用 Rust 和 ratatui 构建的现代化 MySQL 终端客户端，支持首屏帮助与 SQL 模式智能提示。

## 亮点特性

- 数据库/表浏览：快速查看库与表列表
- 表结构/数据查看：结构、10 行数据预览（左右/上下滚动）
- SQL 查询执行：支持常见查询与非查询语句
- SQL 模式智能提示：库名/表名/列名与 SQL 关键字的上下文联想
- 首屏帮助：启动与按 q 返回根目录时统一展示帮助与 INSTRUCTIONS
- 键盘导向：全程键盘操作，快捷键一致清晰
- 跨平台发布：GitHub Releases 自动产物（Linux/macOS/Windows）

## 安装

### 方式一：下载二进制

- 访问仓库 Releases 页面，选择对应平台并下载：
  - `sqltui-rs-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
  - `sqltui-rs-vX.Y.Z-aarch64-apple-darwin.tar.gz`
  - `sqltui-rs-vX.Y.Z-x86_64-pc-windows-msvc.zip`
- 解压后直接运行可执行文件 `sqltui-rs`（Windows 为 `sqltui-rs.exe`）。

### 方式二：源码构建

前置：Rust 1.70+，可访问的 MySQL 服务器。

```bash
git clone <repository-url>
cd sqltui
cargo build --release
# 可执行文件位于 target/release/sqltui-rs
```

## 启动与连接

```bash
# 使用默认参数（localhost:3306, 用户 root）
./target/release/sqltui-rs

# 指定连接参数
./target/release/sqltui-rs -h localhost -u root -p your_password

# 指定数据库
./target/release/sqltui-rs -h localhost -u root -p root123 -d testdb
```

命令行参数：

```
-h, --host <HOST>        MySQL 主机地址 (默认: localhost)
-P, --port <PORT>        MySQL 端口 (默认: 3306)
-u, --username <USER>    用户名 (默认: root)
-p, --password <PASS>    密码 (默认: 空)
-d, --database <DB>      指定数据库 (可选)
```

## 使用说明

### 首屏

- 启动即显示帮助与 INSTRUCTIONS；在任意层级按 `q` 回根目录时同样显示该页面。

### 布局

```
┌─────────────────────────────────────────────────────────┐
│ [SQLTUI] READY | DB: testdb | <Driver>: <Version>     │ ← 状态栏
├─────────────────┬───────────────────────────────────────┤
│ 数据库列表      │ 主内容区域                            │
│ • testdb        │ 帮助/库表信息/表结构/查询结果          │
│ • mysql         │                                       │
│ • sys           │                                       │
├─────────────────┴───────────────────────────────────────┤
│ [CMD_MODE] > mysql>                                    │ ← 输入栏
└─────────────────────────────────────────────────────────┘
```

### 快捷键（全局）

| 按键 | 功能 |
|------|------|
| `↑/↓` | 上下导航 |
| `Enter` | 选择/确认（在 SQL 模式中执行语句） |
| `Esc` | 返回上一级（在 SQL 模式中退出 SQL 模式） |
| `d` | 查看数据库详情 |
| `t` | 查看表详情 |
| `s` | 切换数据库 |
| `:` | 进入 SQL 模式 |
| `q` | 在根目录退出程序 |

### SQL 模式

- 回车执行当前语句，保持在 SQL 模式
- 末尾添加 `\G` 或 `\g` 使用垂直输出
- 输入 `\h` 或 `\help` 显示帮助
- 智能提示：
  - 输入 `use ` 提示库名（可按前缀过滤）
  - 输入 `from `/`join `/`desc `/`describe ` 提示表名（懒加载当前库的表）
  - 输入 `where `/`and `/`or ` 提示列名
  - 输入 `<table>.` 提示该表列名（自动加载并缓存）
  - 上/下或左/右 切换建议；Tab 应用当前建议；Esc 关闭建议
- 历史记录：当建议关闭时，`↑/↓` 在历史命令中切换
- 退出：按 `Esc` 退出 SQL 模式；输入 `exit`/`quit`/`\q` 并回车可退出程序

## 发布与下载

- 打 tag 即生成对应 Release 与跨平台产物：

```bash
git tag v0.1.0
git push origin v0.1.0
```

- 工作流：`.github/workflows/release.yml`
  - 触发条件：`push` 到 `v*` 标签
  - 平台：Linux x86_64 / macOS arm64 / Windows x86_64
  - 产物命名：`sqltui-rs-<TAG>-<TARGET>.(tar.gz|zip)`

## 项目结构

```
src/
├── main.rs          # 程序入口（panic 安全清理、参数解析）
├── config/          # 配置管理（clap 参数、DSN 构造）
├── db/              # 数据库连接与查询（sqlx）
├── models/          # 数据模型
└── ui/              # TUI 界面
    ├── app.rs      # 主应用逻辑（状态机、SQL 模式、智能提示）
    └── components/ # UI 组件（Sidebar/Content/Input/StatusBar）
```

## 开发常用命令

```bash
cargo build           # 开发构建
cargo build --release # 发布构建
cargo test            # 测试
cargo clippy          # 代码检查
```

## 许可证

MIT License（详见 `LICENSE`）
