# Rust CLI 开发进度

## 仓库

- Rust 仓库：`zjing123/redmine-cli-rs`
- CLI 命令名：`redmine-cli`
- Python 版本继续保存在 `zjing123/python-redmine-cli`
- Rust 代码位于仓库根目录，不再使用 `rust/` 子目录

## 已完成

- [x] 创建独立 Rust Cargo 项目
- [x] 复用 `~/.config/redmine-cli/config.yaml` 多 profile 配置
- [x] 支持 `-p/--profile` 和 `REDMINE_PROFILE`
- [x] 支持 Issue 数字 ID 和完整 URL 引用
- [x] 使用 `redmine-api` 实现 Issue API 访问
- [x] 实现 `issue get`
- [x] 实现 `issue list`
- [x] 实现 `issue create`
- [x] 实现 `issue update`
- [x] 实现 `issue delete`
- [x] 实现 Issue `fields` 和 `schema`
- [x] 按资源拆分 Rust 代码目录
- [x] 实现 project、user、time-entry、wiki-page 基础资源命令
- [x] 为各资源添加独立 metadata 文件
- [x] 支持 `--json`、`--stdin` 和 `--fields`
- [x] 实现 `config` profile 管理命令
- [x] 所有资源统一通过 `redmine-api` 使用 API Key
- [x] 实现 `config test` 当前用户认证测试
- [x] 实现 Search API 和 `search <query>` 命令
- [x] 根目录 `cargo check --offline` 编译通过

## 当前命令

```text
redmine-cli issue get|list|create|update|delete|fields|schema
redmine-cli project get|list|create|update|delete|fields|schema
redmine-cli user get|list|create|update|delete|fields|schema
redmine-cli time-entry get|list|create|update|delete|fields|schema
redmine-cli wiki-page get|list|create|update|delete|fields|schema
redmine-cli search <query>
```

## 尚未完成

- [ ] 评估并实现 `--all-profiles`
- [ ] 评估并实现 `--dry-run`
- [ ] 为 Wiki Page 适配 Redmine 专用项目/wiki endpoint
- [ ] 校验各资源通用 CRUD 与不同 Redmine 版本的 endpoint 差异
- [ ] 完善资源级参数和 API 响应模型
- [ ] 补充真实 Redmine 环境验证

## 验证约定

开发期间至少执行：

```bash
cargo fmt --all
cargo check --offline
```

测试暂不作为当前开发阶段的完成条件；涉及真实 Redmine 的验证只执行只读命令。
