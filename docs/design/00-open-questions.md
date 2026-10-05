# Renyi 设计决策：第一轮待决问题

> 状态：等待回答　｜　日期：2026-10-05　｜　回答后整理为 `01-decisions.md`

每题都附了**推荐**默认值。可以逐条按编号回复，也可以回复"全部按推荐，除了 A1、C3 …"。
标 ★ 的问题会改变整体架构方向，请务必回答；其余不回复即视为接受推荐。

---

## 0. 先指出需求之间的四个张力

这几条是需求之间互相拉扯的地方。每条都给了解法，请确认接受这些取舍。

1. **英文化 vs. token 成本与"顺口编造"。** 语法越像自然英语，LLM 越容易按英语直觉写出不存在的句式，生成也越费 token。解法：做"规则的英文"而不是"自然的英文"。固定句式，每个概念只有一种写法，没有同义词，格式化器只有一种输出。
2. **脚本的轻快 vs. Haskell 的严格。** 严格静态类型意味着没有"随手写一行就跑"的动态逃生舱。解法：函数体内全量类型推导，只有顶层签名必须写出，报错信息直接给出修复建议，让 agent 一轮内自我修正。
3. **兼容其他语言 vs. 严格性。** FFI 边界是严格性的漏洞。解法：所有外部调用必须声明类型，在边界做运行时校验，校验失败走正常的错误通道。外部调用是"二等但安全"。
4. **运行速度 vs. 编译速度 vs. 实现工作量。** 不可能同时最优。默认排序：编译/检查速度 > 实现工作量 > 运行速度。v1 先用字节码 VM，热路径后期再做 AOT。

## A. 定位 ★

**A1 ★ 脚本语言、应用语言，还是统一？**
- 脚本：`renyi run x.ry` 秒级启动，标准库齐全，给 agent 做自动化和胶水。
- 应用：编译成可部署产物，如单文件可执行、WASM、长期运行的服务。
- 推荐：一条工具链两种模式。`run` 走字节码 VM 即时执行，`build` 产出自包含可执行文件，后期再加 AOT。

**A2 ★ 第一批真实用例。** 请给 3 个具体场景。例如：agent 调 HTTP API 并处理 JSON 的工作流；数据清洗脚本；后端服务；CLI 工具；让 agent 安全执行计算的沙箱。用例决定标准库优先级和性能目标。

**A3 代码的主要作者是谁？**
- (a) 主要由 LLM agent 写、人类审阅；(b) 人类写、LLM 辅助；(c) 两者对等。
- 推荐：按 (a) 设计，以"人类审阅者一眼读懂"为验收标准。这决定了报错要机器可解析、语法禁止多种写法、文档字段强制。

**A4 性能目标，请给可量化的锚点。** 例如"比 CPython 快 5 到 10 倍即可"、"在 Go 的 3 倍以内"、"启动小于 50 毫秒"。
- 推荐 v1：解释器量级对标 Lua，启动小于 30 毫秒，不追求 v1 就比肩 Rust 或 Go。

**A5 平台优先级排序。** 候选：Linux/macOS/Windows 命令行、浏览器 WASM、服务端 WASI、移动端、嵌入进其他程序（像 Lua 那样）。
- 推荐：三大桌面命令行 + WASI 为一等公民，浏览器二等，移动端暂不考虑。

## B. 类型与严格性（Haskell 的那部分）★

**B1 ★ 副作用是否进入类型系统？** 也就是用英文声明函数的能力，例如 `needs: network, filesystem.read`。
- 好处：agent 一眼看出函数会做什么；运行时据此沙箱化；RAG 可以按"所有写磁盘的函数"检索；把时间和随机数也算作效果后，纯函数天然可重放、可测试。
- 代价：编译器内部复杂度高，但这正是"包进语言内部"的部分。
- 推荐：要。

**B2 错误处理：无异常的 Result 模型，还是异常？**
- 推荐：无异常。表面语法用英文 `or fails with` / `otherwise fail`，`Result` 和 `Maybe` 类型存在于内部但用户很少直接写。只有显式 `crash` 能让程序终止。这是 Elm、Gleam、Roc 那种"无运行时异常"的稳定性来源。

**B3 可变性。**
- 推荐：默认不可变；`let mutable` 显式开放局部可变；不允许可变状态跨函数共享。保留 `for each` 命令式循环，因为 LLM 写命令式循环比写 fold 更稳。

**B4 求值策略。** Haskell 默认惰性，是它难懂和内存泄漏的主要来源。
- 推荐：严格求值，`lazy` 关键字按需开启。

**B5 类型系统特性清单。** 请勾选接受的范围：
- 代数数据类型 + 模式匹配。推荐：要。
- 泛型，语法 `List of Item`。推荐：要。
- 类型类 / trait。推荐：要，叫 `ability`，不向用户暴露高阶类型。
- 高阶类型、GADT、依赖类型。推荐：不暴露，v1 连内部都不做。
- 精炼类型 / 契约，如 `Integer where value > 0`。推荐：v2 再议。
- 类型推导：顶层函数签名强制写出，函数体内全部推导。推荐：是，强制签名是为了 RAG 和可读性。

**B6 ★ 是否接受"零逃生舱"？** 没有 `any`、没有 unsafe cast、没有动态类型模式。
- 推荐：接受。不安全性全部关在 FFI 边界之外。

**B7 空值。** 完全禁止 null，只有 `maybe T`。推荐：是。

**B8 面向对象？** 类、继承、方法重写。
- 推荐：不要类和继承。只有记录类型 + 代数数据类型 + ability。`user.name`、`list.length` 这种成员访问语法保留，因为所有主流语言都这么写。

**B9 数值。**
- 推荐：`Integer` 任意精度，`Decimal` 精确小数，`Float` 为 IEEE 双精度且必须显式选择。不做隐式数值转换。

## C. 语法风格 ★

同一个函数的三种写法。请选一个方向，可以说"2 但 X 改成 1 的写法"。

**样例 1：句子式**
```
define function active_adult_emails
  taking path as Path
  returning List of Email, or failing with FileError
  purpose: Read users from a JSON file and return the emails of active users who are at least 18.
  needs: filesystem read

  let text be read_file(path), otherwise fail
  let users be parse_json(text) as List of User, otherwise fail with InvalidFormat
  let emails be
    for each user in users
    where user.is_active and user.age is at least 18
    collect user.email
  return sort(emails)
end function
```

**样例 2：关键词结构式（推荐）**
```
function active_adult_emails(path: Path) returns List of Email or fails with FileError
  purpose: Read users from a JSON file and return the emails of active users who are at least 18.
  needs: filesystem.read

  let text = read_file(path) otherwise fail
  let users = parse_json(text) as List of User otherwise fail with InvalidFormat
  let emails = for each user in users
               where user.is_active and user.age >= 18
               collect user.email
  return sort(emails)
end
```

**样例 3：近主流式，只把晦涩符号换成词**
```
function active_adult_emails(path: Path) -> Result[List[Email], FileError]:
    purpose: "Read users from a JSON file and return the emails of active users who are at least 18."
    needs: [filesystem.read]

    text = read_file(path)?
    users = parse_json[List[User]](text) or fail(InvalidFormat)
    emails = [user.email for user in users if user.is_active and user.age >= 18]
    return Ok(sorted(emails))
```

**C1 ★ 选哪个方向？**

**C2 ★ 块结构。** 显式 `end`、Python 式缩进、还是花括号？
- 推荐：显式 `end`。理由：LLM 对缩进不够鲁棒；RAG 切片和复制粘贴会破坏缩进；`end` 让每个块自描述。可选允许 `end function` 这类带名字的结尾。

**C3 ★ 符号保留边界。** 提案如下，请指出不同意的行：

| 概念 | 主流写法 | Renyi 提案 |
|---|---|---|
| 绑定 | `=` | 保留 `=` |
| 相等 | `==` `!=` | `is` / `is not`（备选 `equals`） |
| 大小比较 | `< > <= >=` | 保留（备选 `is at least` 等英文） |
| 逻辑 | `&& \|\| !` | `and` `or` `not` |
| 四则 | `+ - * /` | 保留；`%` 改 `remainder`，`**` 改 `power` |
| 返回类型 | `->` | `returns` |
| 泛型 | `List<T>` | `List of T` |
| 可选值 | `T?` | `maybe T` |
| 错误 | `Result<T,E>` + `?` | `or fails with E` + `otherwise fail` |
| 管道 | `\|>` | 不提供，用 let 绑定分步 |
| 匿名函数 | `x => x+1` | 见 C9 |
| 字符串插值 | `${x}` | `"Hello {name}"` |
| 注释 | `//` `#` | `#` |
| 成员与调用 | `a.b` `f(x)` | 保留 |

关键待定：相等用 `is` 还是 `equals`；大小比较保留符号还是英文。

**C4 "一件事只有一种写法"。** 接受禁止同义词吗？只有 `otherwise` 没有 `else`，只有 `function` 没有 `fn`、`def`。
- 推荐：接受。这是防止 LLM 自造语法的核心手段。

**C5 命名规则由编译器强制。** 变量和函数 snake_case，类型 PascalCase，不合规直接报错而不是警告。推荐：是。

**C6 Unicode 标识符，比如中文变量名。**
- 推荐：不允许，字符串和注释里随意。理由：RAG 一致性、工具链简单、避免同形字。请确认。

**C7 一行一句、无分号，行宽由格式化器处理。** 推荐：是。

**C8 ★ 文档与元数据是否强制？**
- 推荐：公开函数必须有 `purpose:` 一句话；模块顶部必须有 `module X purpose: ...`；可选 `example:` 块且作为测试执行。这是"代码可被 RAG 检索"的核心机制。
- 追问：要不要把 `tags:` 和 `see also:` 也做成语言内置字段？

**C9 匿名函数。**
- A：`function(n) n * 2 end`，与普通函数一致但啰嗦。
- B：`(n) gives n * 2`，仅限单表达式。
- C：不支持匿名函数，所有逻辑必须命名。最可读、最利于 RAG，但最啰嗦。
- 推荐：B，且多行逻辑必须抽成命名函数。

## D. 面向 LLM 的约束 ★

**D1 ★ 速查表 token 预算。** 新语言没有训练数据，LLM 只能靠上下文里的规范学习。接受"整个语法速查表必须能塞进一个 prompt，比如不超过 3000 token"作为硬约束吗？
- 推荐：接受，并作为验收指标，每次语法变更后重测。

**D2 冷启动策略。** (a) 规范 + 速查表进上下文；(b) 与主流语言足够相似以便迁移；(c) 后期生成大量示例语料做微调。
- 推荐：a + b 为主，c 为后期。

**D3 报错格式。** 人类可读 + 机器可解析 JSON 双输出，附"建议修复"，`renyi check --json` 是一等公民。推荐：是。

**D4 禁用清单。** v1 禁止：运算符重载、宏和元编程、隐式类型转换、通配符导入、变量遮蔽、单字母变量名（数学公式除外）、嵌套深度超过 4 层。
- 推荐：全部禁止。请勾出有异议的。

**D5 RAG 可检索的操作性定义。** 满足这几条：每个定义自带自然语言 purpose；无通配符导入、无隐式上下文，切片后仍可独立理解；顶层签名和效果声明齐全，可结构化过滤；无运算符重载和宏，文本即语义；工具链提供 `renyi index`，为每个定义输出稳定 ID、签名、purpose、effects。
- 推荐：接受这个定义。Unison 式内容寻址 v1 不做进语言。

**D6 agent 原生特性。** 要不要让"把函数导出为 agent 可调用的工具"成为一等公民？签名 + purpose 自动生成 JSON Schema，一行 `expose as tool` 就能挂到 MCP。
- 推荐：要，这几乎是强制 purpose 和强制签名的免费副产品。

## E. 并发与运行时

**E1 并发模型。** async/await、结构化并发、Actor、Go 式绿色线程，或 v1 单线程。
- 推荐：结构化并发，英文语法 `run concurrently ... end`，内部绿色线程，禁止裸线程和共享可变状态。

**E2 内存管理。** GC、引用计数、所有权。
- 推荐：GC。绝不把所有权暴露给用户。

**E3 沙箱。** 运行时按声明的能力强制执行，像 Deno 的权限模型。推荐：是。

## F. 互操作与"兼容性" ★

**F1 ★ "其他常见语言兼容性"具体指哪些？** 请排优先级：
- (a) 从 Renyi 调用 Python / JS / C 库
- (b) 把 Renyi 嵌入到 Python / JS / Rust 程序里当脚本引擎
- (c) Renyi 转译到其他语言
- (d) 轻松消费 JSON / HTTP / OpenAPI / 数据库
- (e) 语义与主流语言接近，便于把现有代码翻译成 Renyi
- 推荐：d > e > a（先 C，再 Python）> b > c。这几项工作量差一个量级。

**F2 ★ "API 友好"指哪个？**
- 调外部 API 方便：HTTP 和 JSON 一等公民，从 OpenAPI / JSON Schema 生成类型。
- 语言自身提供 API：编译器即库、AST 以 JSON 暴露、LSP、MCP 工具。
- 推荐：都要，请排序。

**F3 最重要的宿主生态。** Python、JS/TS、C、JVM、Go、Rust。

## G. 模块、包、标准库

**G1 包管理。** 内建、中心仓库、lockfile。要不要 Elm 式"编译器对比 API 自动决定大版本号"？推荐：要，这是稳定性承诺的一部分。

**G2 标准库范围。** 电池齐全（HTTP、JSON、文件、时间、正则、CSV）还是最小核心？推荐：小核心 + 同仓库维护、同步发版的官方扩展包。

**G3 文件与模块。** 一文件一模块，模块名等于路径。`.renyi` 和 `.ry` 完全等价。推荐：是。

## H. 实现策略

**H1 ★ 实现语言。** 沙箱里现有 Rust 1.97、Node 22、Bun、Python 3.11。
- Rust：单二进制、WASM、性能好、开发慢。
- Go：开发快、跨编译易、性能上限低。
- OCaml / Haskell：写编译器最顺手、分发麻烦、沙箱里没有。
- TypeScript：原型最快、运行慢。
- 推荐：Rust。可选先用 TypeScript 做两周语法原型验证可读性。追问：你自己最熟什么？你会亲自写实现代码，还是主要由 Claude 在这些会话里写？

**H2 先规范还是先代码？**
- 推荐：先写 30 个示例程序 + 语法速查表，跑一轮"LLM 可读性测试"：给多个模型看样例，让它们预测输出和补全代码，统计正确率，通过后再写解析器。

**H3 里程碑。** M0 设计文档 + 示例语料；M1 词法语法 + 格式化器；M2 类型检查 + 报错；M3 字节码 VM；M4 标准库 + 包管理；M5 LSP + index；M6 AOT / WASM。请说第一个可演示节点和时间盒。

**H4 开源协议。** 推荐 Apache-2.0。

**H5 文档语言。** 推荐规范用英文，设计讨论可中文。

**H6 测试策略。** 规范一致性测试集、golden 测试、property 测试。推荐：全要，从 M1 开始。

## I. 命名与澄清

**I1 "Renyi"的含义。** 是"任意"，还是数学家 Rényi？这影响 logo 和叙事，也影响搜索："Rényi entropy" 会和语言名撞车。请确认拼写固定为 `Renyi`，无重音。

**I2 "关键词处理"具体指什么？** 候选：保留字数量与冲突（变量名叫 `end` 或 `where` 怎么办）；上下文关键字；关键词本地化。默认理解是第一项，解法是小而固定的保留字表 + 冲突时报错并给改名建议。

**I3 "格式优化"具体指什么？** 候选：唯一官方格式化器；面向 token 效率的代码布局；源码编码规范。

**I4 参考语言好恶。** 喜欢或讨厌哪些语言的哪些地方？默认借鉴：Elm 的报错与版本机制、Gleam 的简洁类型系统、Roc 和 Koka 的效果系统、Unison 的内容寻址、Lua 和 Ruby 的 `end`、SQL 的英文关键词；警惕 AppleScript 和 Inform 7 的"易读难写"陷阱。

## 下一步

收到回答后产出：`01-decisions.md` 决策记录、`02-syntax-sketch.md` 语法草案、10 到 30 个示例程序、速查表及其 token 计数。之后进入 M1。
