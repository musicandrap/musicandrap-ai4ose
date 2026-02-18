//! 构建脚本：为 RISC-V64 目标自动生成链接脚本。
//!
//! 链接脚本控制程序各段在内存中的布局，确保：
//! - M-mode 代码（tg-sbi）从 0x80000000 开始
//! - S-mode 代码（_start 入口）从 0x80200000 开始
/*
逐行解析
1. 条件判断
rust
if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
env::var("CARGO_CFG_TARGET_ARCH")：读取 Cargo 设置的环境变量，获取当前编译目标的架构

unwrap_or_default()：如果变量不存在，返回空字符串

== "riscv64"：只有目标架构是 RISC-V 64 时才执行

为什么：在主机（x86_64）上运行 cargo publish 或测试时，不需要生成 RISC-V 的链接脚本

2. 生成链接脚本文件
rust
let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
fs::write(&ld, LINKER_SCRIPT).unwrap();
env::var_os("OUT_DIR")：获取 Cargo 的输出目录（每个包独立的临时目录）

join("linker.ld")：拼接出完整路径 [OUT_DIR]/linker.ld

fs::write(&ld, LINKER_SCRIPT)：将 LINKER_SCRIPT 常量中的内容写入这个文件

LINKER_SCRIPT：应该是在文件其他地方定义的字符串常量，包含实际的链接脚本内容

3. 告诉 Rustc 使用它
rust
println!("cargo:rustc-link-arg=-T{}", ld.display());
println!("cargo:...")：这是构建脚本通知 Cargo 的特殊语法

cargo:rustc-link-arg：告诉 Cargo 向 rustc 传递链接器参数

-T{}：链接器参数 -T 指定使用哪个链接脚本

效果：最终链接时，rustc 会调用链接器并传递 -T[OUT_DIR]/linker.ld，让链接器按照生成的脚本来布局内存

 */

fn main() {
    use std::{env, fs, path::PathBuf};

    if env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "riscv64" {
        let ld = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("linker.ld");
        fs::write(&ld, LINKER_SCRIPT).unwrap();
        println!("cargo:rustc-link-arg=-T{}",ld.display());
    }
}


const LINKER_SCRIPT: &[u8] = b"
OUTPUT_ARCH(riscv)
ENTRY(_m_start)

/* M-mode code base address: start of RAM on QEMU virt platform */
M_BASE_ADDRESS = 0x80000000;
/* S-mode code base address: M-mode jumps here after init */
S_BASE_ADDRESS = 0x80200000;

SECTIONS {
    /* ===== M-mode region (provided by tg-sbi) ===== */
    . = M_BASE_ADDRESS;
    .text.m_entry : { *(.text.m_entry) }
    .text.m_trap  : { *(.text.m_trap)  }
    .bss.m_stack  : { *(.bss.m_stack)  }
    .bss.m_data   : { *(.bss.m_data)   }

    /* ===== S-mode region (this program) ===== */
    . = S_BASE_ADDRESS;
    .text   : {
        *(.text.entry)          /* _start entry, must come first */
        *(.text .text.*)        /* other code */
    }
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data   : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss    : {
        *(.bss.uninit)          /* stack space */
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
}";
