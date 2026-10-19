# Redmine CLI

基于 Rust 构建的 Redmine 命令行工具，专为脚本和 AI Agent 调用设计。命令成功结果以 JSON 输出，支持多 profile 配置和 Redmine API Key 认证。

## 特性

- Rust 编译的独立 CLI，无 Python 运行时依赖
- 多实例 profile 配置
- Issue 获取、列表、创建、更新和删除
- Attachment 下载到本地
- Project、User、Time Entry、Wiki Page 基础 CRUD
- `--fields` 字段投影，减少 Agent token 消耗
- `fields` 和 `schema` 离线元数据查询
- Issue 数字 ID 或完整 URL 引用
- `--json` 和 `--stdin` JSON 输入

## 安装

需要 Rust stable toolchain：

```bash
git clone https://github.com/zjing123/redmine-cli-rs.git
cd redmine-cli-rs
cargo build --release
mkdir -p ~/.local/bin
install -m 755 target/release/redmine-cli ~/.local/bin/redmine-cli
```

开发时可以直接运行：

```bash
cargo run -- --help
redmine-cli --version
```

## 快速开始

### 配置连接

CLI 默认读取 `~/.config/redmine-cli/config.yaml`：

```yaml
profiles:
  prod:
    url: https://redmine.example.com/
    api_key: your_api_key_here
    read_only: true
  staging:
    url: https://staging.example.com/
    key: another_api_key_here
read_only: true
active_profile: prod
```

全局 `read_only: true` 会拒绝对 Redmine 执行创建、更新和删除；profile 中的 `read_only: true` 或 `false` 可以覆盖全局值。省略 profile 级设置时继承全局值。查询、搜索、附件下载及 `fields`/`schema` 不受影响，本地认证配置管理也不受影响。

也可以使用 `REDMINE_CONFIG` 指定配置文件：

```bash
REDMINE_CONFIG=/path/to/config.yaml redmine-cli -p prod issues list
```

### 选择 profile

```bash
redmine-cli -p prod issues list
REDMINE_PROFILE=prod redmine-cli issues list
redmine-cli issues list # 使用 active_profile
```

profile 选择优先级为命令行 `-p/--profile`、环境变量 `REDMINE_PROFILE`、配置中的 `active_profile`。

数字 ID 需要指定 profile；Issue 完整 URL 会自动匹配 profile：

```bash
redmine-cli -p prod issues get 123
redmine-cli issues get https://redmine.example.com/issues/123
```

### 只读示例

```bash
redmine-cli -p prod issues list --project-id 1709 --limit 20
redmine-cli -p prod issues get 123 --fields id,subject,status
```

## 全局选项

```text
-p, --profile <PROFILE>    选择配置 profile；否则使用 REDMINE_PROFILE 或 active_profile
-h, --help                 显示帮助
-V, --version              显示版本
```

全局选项可以放在资源子命令之前，例如：

```bash
redmine-cli -p prod issues get 123
```

## 配置

每个 profile 至少需要以下字段：

| 字段 | 说明 |
| --- | --- |
| `url` | Redmine 服务地址 |
| `api_key` | Redmine API Key |
| `key` | `api_key` 的兼容别名 |
| `read_only` | 可选；限制该 profile 的创建、更新和删除操作，覆盖全局设置 |

认证管理命令：

```bash
redmine-cli auth login
redmine-cli auth login --name work -k YOUR_KEY -s https://redmine.example.com/
redmine-cli auth list
redmine-cli auth status
redmine-cli auth switch work
redmine-cli auth logout work
redmine-cli config path
```

`auth login` 在提供 `-k/--api-key` 和 `-s/--server` 时直接验证并保存，不进入交互窗口；缺少任一参数时会交互补齐。省略 `--name` 时根据服务器主机名生成名称。同名 profile 会被重新验证并更新。`auth list` 不会输出 API Key；`auth status` 会连接 Redmine 获取当前用户信息。`auth switch` 设置默认 profile，`auth logout` 删除 profile 前会交互确认。

多实例配置示例：

```yaml
profiles:
  redminex:
    url: https://redminex.example.com/
    api_key: key_for_redminex
  prod:
    url: https://redmine.example.com/
    api_key: key_for_prod
```

## 命令参考

当前 CLI 按资源组织命令，不再提供旧版本的动态 `resource` 路由。

除 `config`、`fields` 和 `schema` 外，访问 Redmine API 的命令都使用
`-p/--profile <PROFILE>`、`REDMINE_PROFILE` 或 `active_profile` 选择 profile。使用完整 Issue URL 时，CLI
可以根据 URL 自动匹配已配置的 profile。

### issues — Issue 操作

#### issues get

```bash
redmine-cli -p prod issues get 123
redmine-cli issues get https://redmine.example.com/issues/123
redmine-cli -p prod issues get 123 --fields id,subject,status
redmine-cli -p prod issues get 123 --include journals,attachments,relations
redmine-cli -p prod issues get 123 --journals --attachments
redmine-cli -p prod issues get 123 --download-attachments ./downloads/issue-123
```

| 选项 | 说明 |
| --- | --- |
| `<id-or-url>` | Issue 数字 ID 或完整 URL |
| `--fields` | 逗号分隔的输出字段 |
| `--include` | 逗号分隔的关联数据 |
| `--journals`、`--children`、`--relations`、`--attachments` | 对应 `--include` 值的快捷开关，可与 `--include` 混用 |
| `--download-attachments <目录>` | 下载该 Issue 的全部附件；自动请求附件数据，已存在的同名文件不会覆盖 |

#### issues list

```bash
redmine-cli -p prod issues list
redmine-cli -p prod issues list --project-id 1709 --status open
redmine-cli -p prod issues list --assignee me
redmine-cli -p prod issues list --assignee 42
redmine-cli -p prod issues list --assignee none
redmine-cli -p prod issues list --subject "登录" --sort updated_on:desc
redmine-cli -p prod issues list --limit 20 --offset 0 --fields id,subject,status
redmine-cli -p prod issues list --filter status_id=1 --filter cf_12=frontend
redmine-cli -p prod issues list --short
```

支持的快捷过滤、通用过滤和分页选项：

```text
--project-id --tracker-id --priority-id --status --assignee --subject
--filter KEY=VALUE（可重复；自定义字段使用 cf_ID=VALUE）
--include --sort --fields --short --limit --offset
```

`--fields` 可指定任意输出字段；`--short` 返回 `id,subject,status,priority,assigned_to,updated_on`，两者不能同时使用。它们都在 CLI 获取 API 数据后进行输出投影，不改变 API 请求的数据量。列表结果包含 `total_count`、`limit`、`offset`、`profile` 和 `issues`。

`--assignee` 接受负责人 ID、`me`（当前 API 用户）或 `none`（未分配）。

#### issues create

```bash
redmine-cli -p prod issues create --project-id 1709 --subject "新 Issue"
redmine-cli -p prod issues create \
  --project-id 1709 --subject "Bug" --description "问题描述" \
  --tracker-id 3 --priority-id 4 --assignee 42
redmine-cli -p prod issues create --json \
  '{"project_id":1709,"subject":"新 Issue","description":"内容"}'
echo '{"project_id":1709,"subject":"新 Issue"}' | \
  redmine-cli -p prod issues create --stdin
```

快捷参数包括 `--project-id`、`--subject`、`--description`、`--tracker-id`、`--priority-id`、`--assignee` 和 `--custom-fields`。与 JSON 同时使用时，快捷参数覆盖同名字段；`--assignee` 接受用户或组 ID。

#### issues update

```bash
redmine-cli -p prod issues update 123 --subject "更新后的标题"
redmine-cli -p prod issues update 123 --assignee 42
redmine-cli -p prod issues update 123 --json '{"status_id":3,"notes":"已解决"}'
echo '{"status_id":3}' | redmine-cli -p prod issues update 123 --stdin
```

#### issues delete

```bash
redmine-cli -p prod issues delete 123
redmine-cli issues delete https://redmine.example.com/issues/123
```

#### issues fields / schema

这两个命令读取 `src/resources/issue/metadata.yaml`，不访问 Redmine：

```bash
redmine-cli issues fields
redmine-cli issues schema
```

### attachments — Attachment 操作

附件下载需要 profile 和附件 ID。默认保存到 `$HOME/Downloads`，目录不存在时自动创建；
目标文件已存在时不会覆盖。

```bash
redmine-cli -p prod issues get 124365 --include attachments
redmine-cli -p prod attachments download 391795
redmine-cli -p prod attachments download 391795 --output ./clipboard.png
```

### projects — Project 操作

```bash
redmine-cli -p prod projects get 1
redmine-cli -p prod projects get 1 --fields id,name,identifier
redmine-cli -p prod projects list --limit 20 --fields id,name,identifier
redmine-cli -p prod projects list --limit 20 --offset 0
redmine-cli -p prod projects create --json '{"name":"Example","identifier":"example"}'
echo '{"name":"Updated"}' | redmine-cli -p prod projects update 1 --stdin
redmine-cli -p prod projects delete 1
redmine-cli projects fields
redmine-cli projects schema
```

### users — User 操作

```bash
redmine-cli -p prod users get 5
redmine-cli -p prod users get 5 --fields id,login,firstname,lastname
redmine-cli -p prod users list --limit 20 --fields id,login,firstname,lastname
redmine-cli -p prod users list --status 1
redmine-cli -p prod users create --json '{"login":"demo","firstname":"Demo","lastname":"User","mail":"demo@example.com"}'
echo '{"firstname":"Updated"}' | redmine-cli -p prod users update 5 --stdin
redmine-cli -p prod users delete 5
redmine-cli users fields
redmine-cli users schema
```

Redmine 的用户列表 API 要求管理员权限；`--status` 可用于筛选用户状态。

### time — Time Entry 操作

```bash
redmine-cli -p prod time get 1
redmine-cli -p prod time get 1 --fields id,issue,hours,spent_on,user
redmine-cli -p prod time list --project-id 1 --limit 20
redmine-cli -p prod time list --user-id 5 --sort spent_on:desc
redmine-cli -p prod time create --json '{"issue_id":123,"hours":1.5}'
echo '{"hours":2}' | redmine-cli -p prod time update 1 --stdin
redmine-cli -p prod time delete 1
redmine-cli time fields
redmine-cli time schema
```

创建工时需要提供 issue 或 project 以及 hours；如果 Redmine 未配置默认工时活动，还需要在 JSON 中提供 `activity_id`。

### wiki — Wiki Page 操作

注意：当前 Wiki 命令复用通用资源路由（`/wiki_pages/...`），而 [Redmine 官方 Wiki Pages REST API](https://www.redmine.org/projects/redmine/wiki/Rest_WikiPages) 使用项目和页面标题构成的路径（`/projects/{project}/wiki/{title}`）。因此下列命令虽然可被 CLI 解析，但目前不能保证适用于标准 Redmine；完整支持需要调整命令实现。

```bash
redmine-cli -p prod wiki get 1
redmine-cli -p prod wiki get 1 --fields id,title,text
redmine-cli -p prod wiki list --project-id 1709 --limit 20
redmine-cli -p prod wiki list --sort title
redmine-cli -p prod wiki create --json '{"title":"Home","text":"内容"}'
echo '{"text":"Updated content"}' | redmine-cli -p prod wiki update 1 --stdin
redmine-cli -p prod wiki delete 1
redmine-cli wiki fields
redmine-cli wiki schema
```

### 通用资源参数

`project`、`user`、`time` 和 `wiki` 提供统一的基础命令：

```text
get <reference> | list | create | update <reference> | delete <reference> | fields | schema
```

示例：

```bash
redmine-cli -p prod <resource> get <id-or-url>
redmine-cli -p prod <resource> list --limit 20 --fields id,name
redmine-cli -p prod <resource> create --json '{"name":"Example"}'
redmine-cli -p prod <resource> update 1 --json '{"name":"Updated"}'
redmine-cli -p prod <resource> delete 1
redmine-cli <resource> fields
redmine-cli <resource> schema
```

通用资源列表参数（是否受目标 Redmine 资源支持，取决于对应 API）：

```text
--project-id --user-id --assigned-to-id --status
--sort --fields --limit --offset
```

创建和更新支持 JSON 或 stdin：

```bash
redmine-cli -p prod projects create --json \
  '{"name":"Example","identifier":"example"}'
echo '{"name":"Updated"}' | redmine-cli -p prod projects update 1 --stdin
```

各资源元数据文件位于：

```text
src/resources/project/metadata.yaml
src/resources/user/metadata.yaml
src/resources/time_entry/metadata.yaml
src/resources/wiki_page/metadata.yaml
```

### search — Search API

使用当前 profile 查询 Redmine 搜索结果：

```bash
redmine-cli -p prod search "关键词"
redmine-cli -p prod search "关键词" --limit 20 --offset 0
```

## 输出格式

成功结果输出 JSON。单条资源示例：

```json
{"id": 123, "subject": "Example issue"}
```

列表结果包含分页信息和资源数组：

```json
{
  "total_count": 1,
  "limit": 50,
  "offset": 0,
  "profile": "prod",
  "issues": []
}
```

错误信息输出到 stderr，并以非零状态码退出。

## Agent 集成

推荐先使用 `fields` 或 `--fields` 减少输出：

```bash
redmine-cli issues fields
redmine-cli -p prod issues get 123 --fields id,subject,status,assigned_to
```

典型只读流程：

```bash
redmine-cli -p prod issues list --project-id 1709 --limit 10 \
  --sort updated_on:desc --fields id,subject,status
redmine-cli issues get https://redmine.example.com/issues/123 \
  --fields id,subject,description,journals
```

创建、更新和删除属于变更操作，Agent 使用前应确认 profile、资源 ID 和请求内容。

## 开发

```bash
cargo fmt --all
cargo check --locked
cargo build --release
```

## 项目结构

```text
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── output.rs
│   └── resources/
│       ├── common.rs
│       ├── issue/
│       ├── project/
│       ├── user/
│       ├── time_entry/
│       └── wiki_page/
└── tests/
```

每个资源拥有独立目录和 metadata 文件。Issue 使用专用 API 实现；其他资源共享通用 CRUD 层，同时保留独立命令入口，便于后续扩展。

## 依赖

- [redmine-api](https://crates.io/crates/redmine-api) — Redmine API 访问和认证
- [clap](https://crates.io/crates/clap) — CLI 参数解析
- [serde](https://crates.io/crates/serde) — JSON/YAML 序列化

## License

Apache-2.0
