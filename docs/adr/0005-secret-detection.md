# 0005 密钥检测只用高置信规则

- 状态：已采纳

## 决定

- `secrets::detect_secret` 只匹配少量高置信、低误报的模式。不做熵值检测，也不检测助记词：两者对普通文本误报率高，用户碰到“这段为什么没被记录”比偶尔漏掉一个密钥更困惑。
- 当前规则（`SECRET_PATTERNS`）：
  - PEM 私钥（无前缀、RSA、EC、DSA、OPENSSH、ENCRYPTED）和 PGP 私钥块。公钥不拦截。
  - AWS access key（`AKIA`、`ASIA`）、JWT、GitHub token（`ghp_` 等前缀和 fine-grained PAT）、Slack token、OpenAI/Anthropic 风格的 `sk-` key。
  - Stripe 正式环境 key（`sk_live_`、`rk_live_`）。测试 key 和可公开的 `pk_live_` 不拦截。
  - Google API key（`AIza` 后正好 35 位）、GitLab personal access token（`glpat-`）、npm token（`npm_` 后正好 36 位）。
  - 带密码的连接串：postgres、postgresql、mysql、mongodb、redis、rediss、amqp、amqps，scheme 可带 `+驱动` 后缀（如 `mongodb+srv`、`mysql+pymysql`），形如 `scheme://user:password@host` 且密码非空，用户名可以为空（`redis://:password@host`）。密码以 `$`、`<`、`{`、`*` 开头时视为模板占位符（`${DB_PASSWORD}`、`<password>`、`{{password}}`、`****`），不拦截；文档示例里的字面量 `password` 仍会拦截。
- 测试里外形接近真实 token 的样例在运行时拼接，避免触发 GitHub push protection。
