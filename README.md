# symbolic-expr

## 项目简介
这是一个用于处理符号表达式的Rust库。该库提供了一系列工具和函数，用于创建、操作和计算符号表达式。

## 主要功能
- 符号表达式的创建和解析
- 表达式的简化与展开
- 符号计算与求值
- 表达式判定恒等与恒不等

## 示例
```rust
use symbolic_expr::Expr;
use std::collections::HashMap;

// 创建变量
let a = Expr::var("a");
let b = Expr::var("b");
let c = Expr::var("c");

// 基本运算示例
let expr1 = a.clone() + b.clone();  // a + b
let expr2 = (a.clone() * b.clone()) / c.clone();  // (a * b) / c
let expr3 = (a.clone() * 2 + b.clone() * 3) / (c.clone() + 4);  // (2a + 3b)/(c + 4)

// 变量替换
let values = HashMap::from([
    ("a", 6),
    ("b", 4),
    ("c", 3)
]);
assert_eq!(expr2.substitute(&values), 8);  // (6 * 4) / 3 = 8

// 表达式等价性判断
let expr4 = (a.clone() + b.clone()) * c.clone();
let expr5 = a.clone() * c.clone() + b.clone() * c.clone();
assert!(expr4 == expr5);  // (a + b)c = ac + bc

// 表达式不恒等判断
let expr6 = a.clone() * b.clone();
let expr7 = a.clone() * a.clone();
assert!(!(expr6 == expr7));  
assert!(!(expr6 != expr7));  // ab ≠ a²

// 特定值相等但不恒等的例子
let expr8 = a.clone() * a.clone() - b.clone() * b.clone();  // a² - b²
let expr9 = (a.clone() + b.clone()) * (a.clone() - b.clone());  // (a+b)(a-b)
let values1 = HashMap::from([
    ("a", 3),
    ("b", 2)
]);
assert_eq!(expr8.substitute(&values1), expr9.substitute(&values1));  // 在特定值下相等
assert!(!expr8.equivalent(&expr9));  // 但不恒等，因为它们是不同的代数形式

// 永远不等的例子
let expr10 = a.clone() + 1;  // a + 1
let expr11 = a.clone();      // a
assert!(expr10 != expr11);   // a + 1 ≠ a 对任意实数 a 都成立，因为它们相差一个常数




// 复杂表达式示例
let complex = ((a.clone() + b.clone()) * c.clone()) / ((a.clone() - b.clone()) * c.clone());
let simplified = complex.simplify();  // 自动简化为 (a + b)/(a - b)

// 部分变量替换
let partial_values = HashMap::from([("a", 4)]);
let result = expr3.partial_substitute(&partial_values).unwrap();
// 结果: (8 + 3b)/(c + 4)
```

## 许可证
本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。