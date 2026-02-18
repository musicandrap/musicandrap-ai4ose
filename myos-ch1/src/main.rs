//! 我的操作系统内核
//! 
//! 这是一个运行在 RISC-V 64 裸机上的最小操作系统内核，
//! 基于 rCore 教程第一章实现。
//! 
//! # 功能
//! - 支持 SBI 调用进行控制台输出
//! - 支持系统关机
//! - 处理 panic
#![no_main]
#![no_std]
#![cfg_attr(target_arch = "riscv64", deny(warnings, missing_docs))]
#![cfg_attr(not(target_arch = "riscv64"), allow(dead_code))]

use tg_sbi::{console_putchar, shutdown};

#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]//禁用 Rust 的名称修饰（Name Mangling），确保编译后生成的符号名就是函数名本身（_start），而不是被修饰过的乱码（如 _ZN3std...）。
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    /*extern "C"
含义：指定这个函数使用 C 语言的调用约定（Calling Convention）。

为什么需要：

与链接器约定一致：当 QEMU 或引导程序跳转到 _start 时，它期望入口点遵循某种标准的调用约定（通常是 C ABI）。使用 extern "C" 确保 Rust 编译器生成的函数符合这个约定（比如参数如何传递、寄存器如何保存）。


与其他语言互操作：虽然这里没有其他语言，但保持 C 约定是最通用的选择。 */
    const STACK_SIZE: usize = 4096;

    #[unsafe(link_section = ".bss.uninit")]//BSS 是 Block Started by Symbol（由符号开始的块）的缩写，这个名字源于古老的汇编器。在现代编程中，它指的是程序中用于存放未初始化或初始化为0的全局/静态变量的内存段。
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];
    
    /*
    
特性	const	static
内存位置	无固定内存位置，每次使用都是复制	有固定的内存地址，整个程序只有一份实例
编译方式	编译时内联展开（类似 C 的 #define）	编译后存在于数据段（.data/.bss）中
可变性	永远不可变（只能绑定到常量表达式）	可以是不可变（static）或可变（static mut）
初始化	只能在编译期常量表达式	可以在编译期或运行时（但必须是 const 兼容的初始化）
泛型	支持	不支持
Drop	不支持（没有析构函数）	不支持（没有析构函数）
引用	可以取引用，但每次可能不同地址	取引用总是得到同一地址
 */
    core::arch::naked_asm!(
        "la sp, {stack} + {stack_size}",/*
        3. {stack} + {stack_size}
这是一个 Rust 内联汇编的格式化占位符：

{stack}：会被替换为 STACK 符号的地址（见后面的 sym STACK）

+ {stack_size}：加上栈的大小

最终效果：如果 STACK 是数组的起始地址（低地址），加上大小后得到栈的顶部地址（高地址）。因为栈向下生长，sp 应该指向顶部。
 */
        "j {main}",
        stack_size = const STACK_SIZE, 
        stack = sym STACK, 
        main = sym rust_main,
    )

}

extern "C" fn rust_main() -> !{
    for c in b"Hello World!\n" {
        /*
        普通字符串 "Hello"：类型是 &str，是 UTF-8 编码的字符串切片

字节字符串 b"Hello"：类型是 &[u8; N]，是 字节数组的引用
console_putchar 函数接受什么参数？从名字看，它应该接受一个 字节（u8） 或 字符（char） 作为要打印的字符

如果使用普通字符串，迭代出来的元素类型是 char（Rust 的 Unicode 标量值，4 字节），不能直接传给期望 u8 的函数 */
        console_putchar(*c);
    }
    shutdown(false)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    /*
    这是一个非常好的问题！&core::panic::PanicInfo 中的 & 涉及 Rust 的所有权、性能优化和 API 设计原则。让我们深入分析：

核心答案：因为只需要读取，不需要拥有
PanicInfo 使用引用 & 的根本原因是：panic 处理函数只需要读取信息来报告错误，不需要获取所有权或修改数据。

详细解释
1. 所有权与借用
rust
fn panic(_info: &core::panic::PanicInfo) -> !  // 借用（只读）
fn panic(_info: core::panic::PanicInfo) -> !   // 获取所有权
如果使用所有权版本：

PanicInfo 必须被移动到 panic 函数中

函数结束后 PanicInfo 被销毁（drop）

但 panic 函数永不返回（-> !），这其实没问题，因为程序终止了

那为什么还坚持用引用？

2. 性能与栈空间
在裸机环境中，栈空间极其宝贵（可能只有几 KB）。

引用传递：只传递一个指针（在 RISC-V 中是 8 字节或 4 字节）

值传递：可能需要复制整个 PanicInfo 结构体到栈上

PanicInfo 内部可能包含：

文件名指针

行号列号

可选的错误消息

其他调试信息

虽然不大，但在栈紧张的内核中，任何多余的复制都应避免。

3. API 设计的一致性
Rust 标准库和核心库的 panic 处理函数签名是：

rust
fn panic_handler(info: &PanicInfo) -> !
这个设计遵循一个原则：只读访问用引用，需要修改用 &mut，需要拥有用值。panic 处理函数只需要读取信息并采取行动（记录日志、关机等），不需要修改 PanicInfo 本身。

4. _info 的 _ 前缀
你注意到参数名是 _info 而不是 info 吗？这里的下划线 _ 也很重要：

_info：告诉编译器“我知道这个参数存在，但我当前没用到它”，抑制未使用变量的警告

你的 panic 函数只是直接 shutdown(true)，根本没读取 _info 内容

如果写成 info 但没使用，Rust 会警告：unused variable: 'info'

有些 panic 实现会使用 info：

rust
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // 实际读取信息并打印
    if let Some(location) = info.location() {
        println!("Panic at {}:{}", location.file(), location.line());
    }
    shutdown(true)
}
5. PanicInfo 的实际定义
看看 PanicInfo 的内部（简化版）：

rust
pub struct PanicInfo<'a> {
    message: Option<&'a fmt::Arguments<'a>>,
    location: Option<&'a Location<'a>>,
    // 其他字段...
}
它内部本来就包含引用（&'a str、&'a Location 等）。如果按值传递 PanicInfo，这些引用仍然指向原来的数据。按值传递并不会复制这些引用的内容，只会复制指针本身。但为了 API 的一致性和未来可能的变更，Rust 仍然选择传递引用。
 */
    shutdown(true)
}


/// 非 RISC-V64 架构的占位模块。
///
/// 提供 `main` 等符号，使得在主机平台（如 x86_64）上也能通过编译，
/// 满足 `cargo publish --dry-run` 和 `cargo test` 的需求。
#[cfg(not(target_arch = "riscv64"))]
mod stub {
    /// 主机平台占位入口
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        0
    }

    /// C 运行时占位
    #[unsafe(no_mangle)]
    pub extern "C" fn __libc_start_main() -> i32 {
        0
    }

    /// Rust 异常处理人格占位
    #[unsafe(no_mangle)]
    pub extern "C" fn rust_eh_personality() {}
}
