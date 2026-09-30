# Config 管理、认证与 Search 设计

## 目标

在 `redmine-api` fork 的 `dev` 分支中增加 Redmine Search API 支持，并让 `redmine-cli-rs` 使用该 fork，提供完整的 `config` 管理命令。

## 范围

### redmine-api

- 同步和异步客户端使用 API Key 认证。
- 保留现有 `Redmine::new(client, url, api_key)` 和异步等价接口的兼容性。
- 完善 `api::search`，支持 Redmine Search REST API 的关键词、资源类型、分页和常用筛选参数，并返回分页结果。
- 复用现有用户 endpoint 支持获取当前认证用户。
- 增加认证请求和 Search endpoint 的测试，不访问真实 Redmine 服务。

### redmine-cli-rs

- `Cargo.toml` 使用 `redmine-api` fork 的 `dev` 分支。
- 配置 profile 使用以下认证方式：

  ```yaml
  profiles:
    api:
      url: https://redmine.example.com/
      api_key: api-key
  ```

- `api_key` 是必填认证字段。
- 新增命令：

  ```text
  config list
  config get <profile>
  config path
  config validate
  config set <profile> --url <url> [--api-key <key>]
  config update <profile> [--url <url>] [--api-key <key>]
  config test <profile>
  config remove <profile> --yes
  search <query>
  ```

- `config set/update` 在未提供 API Key 时从终端隐藏输入；非交互终端缺少凭据时返回错误。
- `config list/get` 不输出 API Key，只显示 profile、URL 和认证类型。
- `config validate` 只检查本地 YAML、URL 和认证字段，不发起网络请求。
- `config test` 调用当前用户 endpoint，返回脱敏的用户信息；它与 `validate` 不重复，前者验证网络连通性和实际认证。
- `config remove` 要求显式 `--yes`，写入前保留原配置的内存内容并以安全方式重写配置文件。
- `search` 使用当前 profile，通过 `redmine-api` 的 Search endpoint 查询并输出 JSON 分页结果。

## 架构

`redmine-api` 的 API Key 认证作为客户端内部请求层的一部分，所有 CLI API 请求都设置 `X-Redmine-API-Key`。Issue、通用资源、`config test` 和 `search` 共用同一套客户端请求逻辑。

CLI 当前 `src/resources/common.rs` 自建 reqwest 客户端，需要迁移为基于 `redmine-api` 的请求路径，避免 Issue 使用库客户端而其他资源绕过库客户端。迁移只覆盖现有资源命令所需的通用 GET/POST/PUT/DELETE 能力，不改变已有命令参数和 JSON 输出契约。

配置文件仍使用现有 `REDMINE_CONFIG` 和默认路径。写入操作创建父目录（不存在时）并使用 YAML 序列化；读取、列出和校验不修改文件。

## 错误与安全

- API Key、密码不进入成功输出、错误文本或调试格式化结果。
- `update` 只更新明确提供的字段，未提供字段保持不变。
- profile 不存在、认证字段不完整、URL 无效、YAML 无效和文件写入失败都返回非零退出码及 JSON 错误 envelope。
- `config test` 的 HTTP 失败只报告状态和可读错误，不回显 Authorization 或 API Key。
- `remove` 没有 `--yes` 时由 clap 拒绝执行，不修改配置文件。

## 测试

- `redmine-api`：验证 API Key header、同步/异步客户端构造、Search 查询参数和响应分页解析。
- CLI 配置单元测试：profile 解析、认证优先级、隐藏输入所需的非交互错误、脱敏输出、set/update/remove 的文件结果。
- CLI 集成测试：`config` 子命令解析、`validate` 不需要网络、`remove` 缺少 `--yes` 会失败、`search` 参数映射和 JSON 输出。
- 使用 mock HTTP，不依赖真实 Redmine 实例。

## 不在本次范围

- OAuth、Basic Auth、个人访问令牌以外的第三方认证、密码管理器集成。
- 自动发布 crates.io 版本。
- 在 CLI 中保留一套独立于 `redmine-api` 的认证实现。
