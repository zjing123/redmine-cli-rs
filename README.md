# Redmine CLI

基于 Rust 构建的 Redmine 命令行工具，专为脚本和 AI Agent 调用设计。所有命令输出均为 JSON，支持多 profile 配置和 Redmine API Key 认证。

## 特性

- Rust 单文件 CLI，无 Python 运行时依赖
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
git clone https://github.com/zjing123/python-redmine-cli.git
cd python-redmine-cli
cargo build --release
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
  staging:
    url: https://staging.example.com/
    key: another_api_key_here
```

也可以使用 `REDMINE_CONFIG` 指定配置文件：

```bash
REDMINE_CONFIG=/path/to/config.yaml redmine-cli -p prod issue list
```

### 选择 profile

```bash
redmine-cli -p prod issue list
REDMINE_PROFILE=prod redmine-cli issue list
```

数字 ID 需要指定 profile；Issue 完整 URL 会自动匹配 profile：

```bash
redmine-cli -p prod issue get 123
redmine-cli issue get https://redmine.example.com/issues/123
```

### 只读示例

```bash
redmine-cli -p prod issue list --project-id 1709 --limit 20
redmine-cli -p prod issue get 123 --fields id,subject,status
```

## 全局选项

```text
-p, --profile <PROFILE>    选择配置 profile，也可以使用 REDMINE_PROFILE
-h, --help                 显示帮助
-V, --version              显示版本
```

全局选项放在资源命令之前：

```bash
redmine-cli -p prod issue get 123
```

## 配置

每个 profile 至少需要以下字段：

| 字段 | 说明 |
| --- | --- |
| `url` | Redmine 服务地址 |
| `api_key` | Redmine API Key |
| `key` | `api_key` 的兼容别名 |

`config set` 和 `config update` 未提供密钥时会在交互终端中隐藏输入；`config list` 和 `config get` 不会输出 API Key。

配置管理命令：

```bash
redmine-cli config path
redmine-cli config list
redmine-cli config get prod
redmine-cli config validate
redmine-cli config set prod --url https://redmine.example.com/ --api-key YOUR_KEY
redmine-cli config update prod --url https://redmine.example.com/
redmine-cli config test prod
redmine-cli config remove prod --yes
```

`validate` 只检查本地配置；`test` 会连接 Redmine 的当前用户接口并验证认证。

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

除 `config`、`fields` 和 `schema` 外，访问 Redmine API 的命令都需要
`-p/--profile <PROFILE>` 或 `REDMINE_PROFILE`。使用完整 Issue URL 时，CLI
可以根据 URL 自动匹配已配置的 profile；其他资源的数字 ID 和 API 操作仍需要明确指定 profile。

### issue — Issue 操作

#### issue get

```bash
redmine-cli -p prod issue get 123
redmine-cli issue get https://redmine.example.com/issues/123
redmine-cli -p prod issue get 123 --fields id,subject,status
redmine-cli -p prod issue get 123 --include journals,attachments,relations
```

| 选项 | 说明 |
| --- | --- |
| `<id-or-url>` | Issue 数字 ID 或完整 URL |
| `--fields` | 逗号分隔的输出字段 |
| `--include` | 逗号分隔的关联数据 |

#### issue list

```bash
redmine-cli -p prod issue list
redmine-cli -p prod issue list --project-id 1709 --status open
redmine-cli -p prod issue list --assigned-to-id 42
redmine-cli -p prod issue list --subject "登录" --sort updated_on:desc
redmine-cli -p prod issue list --limit 20 --offset 0 --fields id,subject,status
redmine-cli -p prod issue list --custom-fields '{"12":"frontend"}'
```

支持的过滤和分页选项：

```text
--project-id --tracker-id --priority-id --status --assigned-to-id
--subject --custom-fields --include --sort --fields --limit --offset
```

`--custom-fields` 接受 JSON 对象，列表结果包含 `total_count`、`limit`、`offset`、`profile` 和 `issues`。

#### issue create

```bash
redmine-cli -p prod issue create --project-id 1709 --subject "新 Issue"
redmine-cli -p prod issue create \
  --project-id 1709 --subject "Bug" --description "问题描述" \
  --tracker-id 3 --priority-id 4 --assigned-to-id 42
redmine-cli -p prod issue create --json \
  '{"project_id":1709,"subject":"新 Issue","description":"内容"}'
echo '{"project_id":1709,"subject":"新 Issue"}' | \
  redmine-cli -p prod issue create --stdin
```

快捷参数包括 `--project-id`、`--subject`、`--description`、`--tracker-id`、`--priority-id`、`--assigned-to-id` 和 `--custom-fields`。与 JSON 同时使用时，快捷参数覆盖同名字段。

#### issue update

```bash
redmine-cli -p prod issue update 123 --subject "更新后的标题"
redmine-cli -p prod issue update 123 --assigned-to-id 42
redmine-cli -p prod issue update 123 --json '{"status_id":3,"notes":"已解决"}'
echo '{"status_id":3}' | redmine-cli -p prod issue update 123 --stdin
```

#### issue delete

```bash
redmine-cli -p prod issue delete 123
redmine-cli issue delete https://redmine.example.com/issues/123
```

#### issue fields / schema

这两个命令读取 `src/resources/issue/metadata.yaml`，不访问 Redmine：

```bash
redmine-cli issue fields
redmine-cli issue schema
```

### attachment — Attachment 操作

附件下载需要 profile 和附件 ID。默认保存到 `$HOME/Downloads`，目录不存在时自动创建；
目标文件已存在时不会覆盖。

```bash
redmine-cli -p prod issue get 124365 --include attachments
redmine-cli -p prod attachment download 391795
redmine-cli -p prod attachment download 391795 --output ./clipboard.png
```

### project — Project 操作

```bash
redmine-cli -p prod project get 1
redmine-cli -p prod project get 1 --fields id,name,identifier
redmine-cli -p prod project list --limit 20 --fields id,name,identifier
redmine-cli -p prod project list --status 1 --sort name --offset 0
redmine-cli -p prod project create --json '{"name":"Example","identifier":"example"}'
echo '{"name":"Updated"}' | redmine-cli -p prod project update 1 --stdin
redmine-cli -p prod project delete 1
redmine-cli project fields
redmine-cli project schema
```

### user — User 操作

```bash
redmine-cli -p prod user get 5
redmine-cli -p prod user get 5 --fields id,login,firstname,lastname
redmine-cli -p prod user list --limit 20 --fields id,login,firstname,lastname
redmine-cli -p prod user list --project-id 1 --status 1
redmine-cli -p prod user create --json '{"login":"demo","firstname":"Demo","lastname":"User"}'
echo '{"firstname":"Updated"}' | redmine-cli -p prod user update 5 --stdin
redmine-cli -p prod user delete 5
redmine-cli user fields
redmine-cli user schema
```

### time-entry — Time Entry 操作

```bash
redmine-cli -p prod time-entry get 1
redmine-cli -p prod time-entry get 1 --fields id,issue,hours,spent_on,user
redmine-cli -p prod time-entry list --project-id 1 --limit 20
redmine-cli -p prod time-entry list --user-id 5 --sort spent_on:desc
redmine-cli -p prod time-entry create --json '{"issue_id":123,"hours":1.5}'
echo '{"hours":2}' | redmine-cli -p prod time-entry update 1 --stdin
redmine-cli -p prod time-entry delete 1
redmine-cli time-entry fields
redmine-cli time-entry schema
```

### wiki-page — Wiki Page 操作

```bash
redmine-cli -p prod wiki-page get 1
redmine-cli -p prod wiki-page get 1 --fields id,title,text
redmine-cli -p prod wiki-page list --project-id 1709 --limit 20
redmine-cli -p prod wiki-page list --sort title
redmine-cli -p prod wiki-page create --json '{"title":"Home","text":"内容"}'
echo '{"text":"Updated content"}' | redmine-cli -p prod wiki-page update 1 --stdin
redmine-cli -p prod wiki-page delete 1
redmine-cli wiki-page fields
redmine-cli wiki-page schema
```

### 通用资源参数

`project`、`user`、`time-entry` 和 `wiki-page` 提供统一的基础命令：

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

通用列表选项：

```text
--project-id --user-id --assigned-to-id --status
--sort --fields --limit --offset
```

创建和更新支持 JSON 或 stdin：

```bash
redmine-cli -p prod project create --json \
  '{"name":"Example","identifier":"example"}'
echo '{"name":"Updated"}' | redmine-cli -p prod project update 1 --stdin
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
redmine-cli issue fields
redmine-cli -p prod issue get 123 --fields id,subject,status,assigned_to
```

典型只读流程：

```bash
redmine-cli -p prod issue list --project-id 1709 --limit 10 \
  --sort updated_on:desc --fields id,subject,status
redmine-cli issue get https://redmine.example.com/issues/123 \
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
