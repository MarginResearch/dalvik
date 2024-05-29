//! [Dalvik bytecode] instruction decoding and basic block analysis.
//!
//! [Dalvik bytecode]: https://source.android.com/docs/core/runtime/dalvik-bytecode
//!
//! The lifted [`Instruction`] type implements [`Display`][`std::fmt::Display`]
//! for printing the instruction mnemonics, however the best disassembly
//! (closely matching baksmali) is possible only with dex metadata available,
//! which can be provided through the [`PrettyPrint`] trait.

#![warn(missing_docs)]

#[cfg(test)]
mod tests;

pub mod blocks;
pub mod decode;

/// Dalvik Instruction
///
/// See the [reference] for instruction semantics
///
/// [reference]: https://source.android.com/docs/core/runtime/dalvik-bytecode
///
/// Most of the enum variants here are simple tuples. The ordering of the
/// contained values matches the order of the instruction mnemonic as read from
/// left to right.
#[derive(Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Instruction {
    Nop,                                                       // 00
    Move(u8, u8),                                              // 01
    MoveFrom16(u8, u16),                                       // 02
    Move16(u16, u16),                                          // 03
    MoveWide(u8, u8),                                          // 04
    MoveWideFrom16(u8, u16),                                   // 05
    MoveWide16(u16, u16),                                      // 06
    MoveObject(u8, u8),                                        // 07
    MoveObjectFrom16(u8, u16),                                 // 08
    MoveObject16(u16, u16),                                    // 09
    MoveResult(u8),                                            // 0a
    MoveResultWide(u8),                                        // 0b
    MoveResultObject(u8),                                      // 0c
    MoveException(u8),                                         // 0d
    ReturnVoid,                                                // 0e
    Return(u8),                                                // 0f
    ReturnWide(u8),                                            // 10
    ReturnObject(u8),                                          // 11
    Const4(u8, i8),                                            // 12
    Const16(u8, i16),                                          // 13
    Const(u8, u32),                                            // 14
    ConstHigh16(u8, i16),                                      // 15
    ConstWide16(u8, i16),                                      // 16
    ConstWide32(u8, u32),                                      // 17
    ConstWide(u8, u64),                                        // 18
    ConstWideHigh16(u8, u16),                                  // 19
    ConstString(u8, u16),                                      // 1a
    ConstStringJumbo(u8, u32),                                 // 1b
    ConstClass(u8, u16),                                       // 1c
    MonitorEnter(u8),                                          // 1d
    MonitorExit(u8),                                           // 1e
    CheckCast(u8, u16),                                        // 1f
    InstanceOf(u8, u8, u16),                                   // 20
    ArrayLength(u8, u8),                                       // 21
    NewInstance(u8, u16),                                      // 22
    NewArray(u8, u8, u16),                                     // 23
    FilledNewArray { ty: u16, nargs: u8, args: [u8; 5] },      // 24
    FilledNewArrayRange { ty: u16, args: Vec<u16> },           // 25
    FillArrayData(u8, i32),                                    // 26
    Throw(u8),                                                 // 27
    Goto(i8),                                                  // 28
    Goto16(i16),                                               // 29
    Goto32(i32),                                               // 2a
    PackedSwitch(u8, i32),                                     // 2b
    SparseSwitch(u8, i32),                                     // 2c
    CmplFloat(u8, u8, u8),                                     // 2d
    CmpgFloat(u8, u8, u8),                                     // 2e
    CmplDouble(u8, u8, u8),                                    // 2f
    CmpgDouble(u8, u8, u8),                                    // 30
    CmpLong(u8, u8, u8),                                       // 31
    IfEq(u8, u8, i16),                                         // 32
    IfNe(u8, u8, i16),                                         // 33
    IfLt(u8, u8, i16),                                         // 34
    IfGe(u8, u8, i16),                                         // 35
    IfGt(u8, u8, i16),                                         // 36
    IfLe(u8, u8, i16),                                         // 37
    IfEqz(u8, i16),                                            // 38
    IfNez(u8, i16),                                            // 39
    IfLtz(u8, i16),                                            // 3a
    IfGez(u8, i16),                                            // 3b
    IfGtz(u8, i16),                                            // 3c
    IfLez(u8, i16),                                            // 3d
    AGet(u8, u8, u8),                                          // 44
    AGetWide(u8, u8, u8),                                      // 45
    AGetObject(u8, u8, u8),                                    // 46
    AGetBoolean(u8, u8, u8),                                   // 47
    AGetByte(u8, u8, u8),                                      // 48
    AGetChar(u8, u8, u8),                                      // 49
    AGetShort(u8, u8, u8),                                     // 4a
    APut(u8, u8, u8),                                          // 4b
    APutWide(u8, u8, u8),                                      // 4c
    APutObject(u8, u8, u8),                                    // 4d
    APutBoolean(u8, u8, u8),                                   // 4e
    APutByte(u8, u8, u8),                                      // 4f
    APutChar(u8, u8, u8),                                      // 50
    APutShort(u8, u8, u8),                                     // 51
    IGet(u8, u8, u16),                                         // 52
    IGetWide(u8, u8, u16),                                     // 53
    IGetObject(u8, u8, u16),                                   // 54
    IGetBoolean(u8, u8, u16),                                  // 55
    IGetByte(u8, u8, u16),                                     // 56
    IGetChar(u8, u8, u16),                                     // 57
    IGetShort(u8, u8, u16),                                    // 58
    IPut(u8, u8, u16),                                         // 59
    IPutWide(u8, u8, u16),                                     // 5a
    IPutObject(u8, u8, u16),                                   // 5b
    IPutBoolean(u8, u8, u16),                                  // 5c
    IPutByte(u8, u8, u16),                                     // 5d
    IPutChar(u8, u8, u16),                                     // 5e
    IPutShort(u8, u8, u16),                                    // 5f
    SGet(u8, u16),                                             // 60
    SGetWide(u8, u16),                                         // 61
    SGetObject(u8, u16),                                       // 62
    SGetBoolean(u8, u16),                                      // 63
    SGetByte(u8, u16),                                         // 64
    SGetChar(u8, u16),                                         // 65
    SGetShort(u8, u16),                                        // 66
    SPut(u8, u16),                                             // 67
    SPutWide(u8, u16),                                         // 68
    SPutObject(u8, u16),                                       // 69
    SPutBoolean(u8, u16),                                      // 6a
    SPutByte(u8, u16),                                         // 6b
    SPutChar(u8, u16),                                         // 6c
    SPutShort(u8, u16),                                        // 6d
    InvokeVirtual { method: u16, nargs: u8, args: [u8; 5] },   // 6e
    InvokeSuper { method: u16, nargs: u8, args: [u8; 5] },     // 6f
    InvokeDirect { method: u16, nargs: u8, args: [u8; 5] },    // 70
    InvokeStatic { method: u16, nargs: u8, args: [u8; 5] },    // 71
    InvokeInterface { method: u16, nargs: u8, args: [u8; 5] }, // 72
    InvokeVirtualRange { method: u16, args: Vec<u16> },        // 74
    InvokeSuperRange { method: u16, args: Vec<u16> },          // 75
    InvokeDirectRange { method: u16, args: Vec<u16> },         // 76
    InvokeStaticRange { method: u16, args: Vec<u16> },         // 77
    InvokeInterfaceRange { method: u16, args: Vec<u16> },      // 78
    NegInt(u8, u8),                                            // 7b
    NotInt(u8, u8),                                            // 7c
    NegLong(u8, u8),                                           // 7d
    NotLong(u8, u8),                                           // 7e
    NegFloat(u8, u8),                                          // 7f
    NegDouble(u8, u8),                                         // 80
    IntToLong(u8, u8),                                         // 81
    IntToFloat(u8, u8),                                        // 82
    IntToDouble(u8, u8),                                       // 83
    LongToInt(u8, u8),                                         // 84
    LongToFloat(u8, u8),                                       // 85
    LongToDouble(u8, u8),                                      // 86
    FloatToInt(u8, u8),                                        // 87
    FloatToLong(u8, u8),                                       // 88
    FloatToDouble(u8, u8),                                     // 89
    DoubleToInt(u8, u8),                                       // 8a
    DoubleToLong(u8, u8),                                      // 8b
    DoubleToFloat(u8, u8),                                     // 8c
    IntTobyte(u8, u8),                                         // 8d
    IntTochar(u8, u8),                                         // 8e
    IntToshort(u8, u8),                                        // 8f
    AddInt(u8, u8, u8),                                        // 90
    SubInt(u8, u8, u8),                                        // 91
    MulInt(u8, u8, u8),                                        // 92
    DivInt(u8, u8, u8),                                        // 93
    RemInt(u8, u8, u8),                                        // 94
    AndInt(u8, u8, u8),                                        // 95
    OrInt(u8, u8, u8),                                         // 96
    XorInt(u8, u8, u8),                                        // 97
    ShlInt(u8, u8, u8),                                        // 98
    ShrInt(u8, u8, u8),                                        // 99
    UshrInt(u8, u8, u8),                                       // 9a
    AddLong(u8, u8, u8),                                       // 9b
    SubLong(u8, u8, u8),                                       // 9c
    MulLong(u8, u8, u8),                                       // 9d
    DivLong(u8, u8, u8),                                       // 9e
    RemLong(u8, u8, u8),                                       // 9f
    AndLong(u8, u8, u8),                                       // a0
    OrLong(u8, u8, u8),                                        // a1
    XorLong(u8, u8, u8),                                       // a2
    ShlLong(u8, u8, u8),                                       // a3
    ShrLong(u8, u8, u8),                                       // a4
    UshrLong(u8, u8, u8),                                      // a5
    AddFloat(u8, u8, u8),                                      // a6
    SubFloat(u8, u8, u8),                                      // a7
    MulFloat(u8, u8, u8),                                      // a8
    DivFloat(u8, u8, u8),                                      // a9
    RemFloat(u8, u8, u8),                                      // aa
    AddDouble(u8, u8, u8),                                     // ab
    SubDouble(u8, u8, u8),                                     // ac
    MulDouble(u8, u8, u8),                                     // ad
    DivDouble(u8, u8, u8),                                     // ae
    RemDouble(u8, u8, u8),                                     // af
    AddInt2(u8, u8),                                           // b0
    SubInt2(u8, u8),                                           // b1
    MulInt2(u8, u8),                                           // b2
    DivInt2(u8, u8),                                           // b3
    RemInt2(u8, u8),                                           // b4
    AndInt2(u8, u8),                                           // b5
    OrInt2(u8, u8),                                            // b6
    XorInt2(u8, u8),                                           // b7
    ShlInt2(u8, u8),                                           // b8
    ShrInt2(u8, u8),                                           // b9
    UShrInt2(u8, u8),                                          // ba
    AddLong2(u8, u8),                                          // bb
    SubLong2(u8, u8),                                          // bc
    MulLong2(u8, u8),                                          // bd
    DivLong2(u8, u8),                                          // be
    RemLong2(u8, u8),                                          // bf
    AndLong2(u8, u8),                                          // c0
    OrLong2(u8, u8),                                           // c1
    XorLong2(u8, u8),                                          // c2
    ShlLong2(u8, u8),                                          // c3
    ShrLong2(u8, u8),                                          // c4
    UShrLong2(u8, u8),                                         // c5
    AddFloat2(u8, u8),                                         // c6
    SubFloat2(u8, u8),                                         // c7
    MulFloat2(u8, u8),                                         // c8
    DivFloat2(u8, u8),                                         // c9
    RemFloat2(u8, u8),                                         // ca
    AddDouble2(u8, u8),                                        // cb
    SubDouble2(u8, u8),                                        // cc
    MulDouble2(u8, u8),                                        // cd
    DivDouble2(u8, u8),                                        // ce
    RemDouble2(u8, u8),                                        // cf
    AddInt16(u8, u8, i16),                                     // d0
    RsubInt16(u8, u8, i16),                                    // d1
    MulInt16(u8, u8, i16),                                     // d2
    DivInt16(u8, u8, i16),                                     // d3
    RemInt16(u8, u8, i16),                                     // d4
    AndInt16(u8, u8, i16),                                     // d5
    OrInt16(u8, u8, i16),                                      // d6
    XorInt16(u8, u8, i16),                                     // d7
    AddInt8(u8, u8, i8),                                       // d8
    RsubInt8(u8, u8, i8),                                      // d9
    MulInt8(u8, u8, i8),                                       // da
    DivInt8(u8, u8, i8),                                       // db
    RemInt8(u8, u8, i8),                                       // dc
    AndInt8(u8, u8, i8),                                       // dd
    OrInt8(u8, u8, i8),                                        // de
    XorInt8(u8, u8, i8),                                       // df
    ShlInt8(u8, u8, i8),                                       // e0
    ShrInt8(u8, u8, i8),                                       // e1
    UshrInt8(u8, u8, i8),                                      // e2
}

/// Describes the possible control flow effects of an [`Instruction`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlFlow {
    /// Terminates the method (throw or return)
    Terminate,
    /// Jumps to the relative address
    GoTo(i32),
    /// Falls through, or jumps to the relative address
    Branch(i16),
    /// Proceeds to the next in sequence
    FallThrough,
}

impl Instruction {
    /// Get the control flow behavior of the instruction
    #[rustfmt::skip]
    pub fn control_flow(&self) -> ControlFlow {
        match self {
            Self::ReturnVoid
            | Self::Return(_)
            | Self::ReturnWide(_)
            | Self::ReturnObject(_)
            | Self::Throw(_) => ControlFlow::Terminate,

            Self::Goto(t) => ControlFlow::GoTo((*t).into()),
            Self::Goto16(t) => ControlFlow::GoTo((*t).into()),
            Self::Goto32(t) => ControlFlow::GoTo(*t),

            Self::IfEq(_, _, t)
            | Self::IfNe(_, _, t)
            | Self::IfLt(_, _, t)
            | Self::IfGe(_, _, t)
            | Self::IfGt(_, _, t)
            | Self::IfLe(_, _, t)
            | Self::IfEqz(_, t)
            | Self::IfNez(_, t)
            | Self::IfLtz(_, t)
            | Self::IfGez(_, t)
            | Self::IfGtz(_, t)
            | Self::IfLez(_, t) => ControlFlow::Branch(*t),

            _ => ControlFlow::FallThrough,
        }
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Nop => f.write_str("nop"),
            Instruction::Move(dst, src) => two_regs_display(f, "move", *dst, *src),
            Instruction::MoveFrom16(dst, src) => two_regs_display(f, "move/from16", *dst, *src),
            Instruction::Move16(dst, src) => two_regs_display(f, "move/16", *dst, *src),
            Instruction::MoveWide(dst, src) => two_regs_display(f, "move-wide", *dst, *src),
            Instruction::MoveWideFrom16(dst, src) => two_regs_display(f, "move-wide/from16", *dst, *src),
            Instruction::MoveWide16(dst, src) => two_regs_display(f, "move-wide/16", *dst, *src),
            Instruction::MoveObject(dst, src) => two_regs_display(f, "move-object", *dst, *src),
            Instruction::MoveObjectFrom16(dst, src) => two_regs_display(f, "move-object/from16", *dst, *src),
            Instruction::MoveObject16(dst, src) => two_regs_display(f, "move-object/16", *dst, *src),
            Instruction::MoveResult(reg) => one_regs_display(f, "move-result", *reg),
            Instruction::MoveResultWide(reg) => one_regs_display(f, "move-result-wide", *reg),
            Instruction::MoveResultObject(reg) => one_regs_display(f, "move-result-object", *reg),
            Instruction::MoveException(reg) => one_regs_display(f, "move-exception", *reg),
            Instruction::ReturnVoid => f.write_str("return-void"),
            Instruction::Return(reg) => one_regs_display(f, "return", *reg),
            Instruction::ReturnWide(reg) => one_regs_display(f, "return-wide", *reg),
            Instruction::ReturnObject(reg) => one_regs_display(f, "return-object", *reg),
            Instruction::Const4(dst, src) => f.write_fmt(format_args!("const/4 v{dst}, {src:#x}")),
            Instruction::Const16(dst, src) => f.write_fmt(format_args!("const/16 v{dst}, {src:#x}")),
            Instruction::Const(dst, src) => f.write_fmt(format_args!("const v{dst}, {src:#x}")),
            Instruction::ConstHigh16(dst, src) => f.write_fmt(format_args!("const/high16 v{dst}, {src:#x}0000")),
            Instruction::ConstWide16(dst, src) => f.write_fmt(format_args!("const-wide/16 v{dst}, {src:#x}")),
            Instruction::ConstWide32(dst, src) => f.write_fmt(format_args!("const-wide/32 v{dst}, {src:#x}")),
            Instruction::ConstWide(dst, src) => f.write_fmt(format_args!("const-wide v{dst}, {src:#x}")),
            Instruction::ConstWideHigh16(dst, src) => f.write_fmt(format_args!("const-wide/high16 v{dst}, {src:#x}")),
            Instruction::ConstString(dst, idx) => f.write_fmt(format_args!("const-string v{dst}, string@{idx:x}")),
            Instruction::ConstStringJumbo(dst, idx) => f.write_fmt(format_args!("const-string/jumbo v{dst}, string@{idx:x}")),
            Instruction::ConstClass(dst, idx) => f.write_fmt(format_args!("const-class v{dst}, type@{idx:x}")),
            Instruction::MonitorEnter(reg) => one_regs_display(f, "monitor-enter", *reg),
            Instruction::MonitorExit(reg) => one_regs_display(f, "monitor-exit", *reg),
            Instruction::CheckCast(reg, ty) => f.write_fmt(format_args!("check-cast v{reg}, type@{ty:x}")),
            Instruction::InstanceOf(dst, src, ty) => f.write_fmt(format_args!("instance-of v{dst}, v{src}, type@{ty:x}")),
            Instruction::ArrayLength(dst, src) => two_regs_display(f, "array-length", *dst, *src),
            Instruction::NewInstance(reg, ty) => f.write_fmt(format_args!("new-instance v{reg}, type@{ty:x}")),
            Instruction::NewArray(dst, size, ty) => f.write_fmt(format_args!("new-array v{dst}, v{size}, type@{ty:x}")),
            Instruction::FilledNewArray { ty, nargs, args } => {
                f.write_fmt(format_args!("filled-new-array {{"))?;
                for (n, arg) in args[..*nargs as usize].iter().enumerate() {
                    match n {
                        0 => f.write_fmt(format_args!("v{arg}"))?,
                        _ => f.write_fmt(format_args!(", v{arg}"))?,
                    }
                }
                f.write_fmt(format_args!("}}, type@{ty:x}"))
            }
            Instruction::FilledNewArrayRange { ty, args } => {
                f.write_fmt(format_args!("filled-new-array/range {{"))?;
                for (n, arg) in args.iter().enumerate() {
                    match n {
                        0 => f.write_fmt(format_args!("v{arg}"))?,
                        _ => f.write_fmt(format_args!(", v{arg}"))?,
                    }
                }
                f.write_fmt(format_args!("}}, type@{ty:x}"))
            }
            Instruction::FillArrayData(array, off) => f.write_fmt(format_args!("fill-array-data v{array}, {off:+}")),
            Instruction::Throw(reg) => one_regs_display(f, "throw", *reg),
            Instruction::Goto(off) => f.write_fmt(format_args!("goto {off:+}")),
            Instruction::Goto16(off) => f.write_fmt(format_args!("goto/16 {off:+}")),
            Instruction::Goto32(off) => f.write_fmt(format_args!("goto/32 {off:+}")),
            Instruction::PackedSwitch(reg, off) => f.write_fmt(format_args!("packed-switch v{reg}, {off:+}")),
            Instruction::SparseSwitch(reg, off) => f.write_fmt(format_args!("sparse-switch v{reg}, {off:+}")),
            Instruction::CmplFloat(dst, src1, src2) => three_regs_display(f, "cmpl-float", *dst, *src1, *src2),
            Instruction::CmpgFloat(dst, src1, src2) => three_regs_display(f, "cmpg-float", *dst, *src1, *src2),
            Instruction::CmplDouble(dst, src1, src2) => three_regs_display(f, "cmpl-double", *dst, *src1, *src2),
            Instruction::CmpgDouble(dst, src1, src2) => three_regs_display(f, "cmpg-double", *dst, *src1, *src2),
            Instruction::CmpLong(dst, src1, src2) => three_regs_display(f, "cmp-long", *dst, *src1, *src2),
            Instruction::IfEq(a, b, off) => f.write_fmt(format_args!("if-eq v{a}, v{b} {off:+}")),
            Instruction::IfNe(a, b, off) => f.write_fmt(format_args!("if-ne v{a}, v{b} {off:+}")),
            Instruction::IfLt(a, b, off) => f.write_fmt(format_args!("if-lt v{a}, v{b} {off:+}")),
            Instruction::IfGe(a, b, off) => f.write_fmt(format_args!("if-ge v{a}, v{b} {off:+}")),
            Instruction::IfGt(a, b, off) => f.write_fmt(format_args!("if-gt v{a}, v{b} {off:+}")),
            Instruction::IfLe(a, b, off) => f.write_fmt(format_args!("if-le v{a}, v{b} {off:+}")),
            Instruction::IfEqz(reg, off) => f.write_fmt(format_args!("if-eqz v{reg}, {off:+}")),
            Instruction::IfNez(reg, off) => f.write_fmt(format_args!("if-nez v{reg}, {off:+}")),
            Instruction::IfLtz(reg, off) => f.write_fmt(format_args!("if-ltz v{reg}, {off:+}")),
            Instruction::IfGez(reg, off) => f.write_fmt(format_args!("if-gez v{reg}, {off:+}")),
            Instruction::IfGtz(reg, off) => f.write_fmt(format_args!("if-gtz v{reg}, {off:+}")),
            Instruction::IfLez(reg, off) => f.write_fmt(format_args!("if-lez v{reg}, {off:+}")),
            Instruction::AGet(dst, src1, src2) => three_regs_display(f, "aget", *dst, *src1, *src2),
            Instruction::AGetWide(dst, src1, src2) => three_regs_display(f, "aget-wide", *dst, *src1, *src2),
            Instruction::AGetObject(dst, src1, src2) => three_regs_display(f, "aget-object", *dst, *src1, *src2),
            Instruction::AGetBoolean(dst, src1, src2) => three_regs_display(f, "aget-boolean", *dst, *src1, *src2),
            Instruction::AGetByte(dst, src1, src2) => three_regs_display(f, "aget-byte", *dst, *src1, *src2),
            Instruction::AGetChar(dst, src1, src2) => three_regs_display(f, "aget-char", *dst, *src1, *src2),
            Instruction::AGetShort(dst, src1, src2) => three_regs_display(f, "aget-short", *dst, *src1, *src2),
            Instruction::APut(dst, src1, src2) => three_regs_display(f, "aput", *dst, *src1, *src2),
            Instruction::APutWide(dst, src1, src2) => three_regs_display(f, "aput-wide", *dst, *src1, *src2),
            Instruction::APutObject(dst, src1, src2) => three_regs_display(f, "aput-object", *dst, *src1, *src2),
            Instruction::APutBoolean(dst, src1, src2) => three_regs_display(f, "aput-boolean", *dst, *src1, *src2),
            Instruction::APutByte(dst, src1, src2) => three_regs_display(f, "aput-byte", *dst, *src1, *src2),
            Instruction::APutChar(dst, src1, src2) => three_regs_display(f, "aput-char", *dst, *src1, *src2),
            Instruction::APutShort(dst, src1, src2) => three_regs_display(f, "aput-short", *dst, *src1, *src2),
            Instruction::IGet(dst, src, field) => igetters_display("iget", f, *dst, *src, *field),
            Instruction::IGetWide(dst, src, field) => igetters_display("iget-wide", f, *dst, *src, *field),
            Instruction::IGetObject(dst, src, field) => igetters_display("iget-object", f, *dst, *src, *field),
            Instruction::IGetBoolean(dst, src, field) => igetters_display("iget-boolean", f, *dst, *src, *field),
            Instruction::IGetByte(dst, src, field) => igetters_display("iget-byte", f, *dst, *src, *field),
            Instruction::IGetChar(dst, src, field) => igetters_display("iget-char", f, *dst, *src, *field),
            Instruction::IGetShort(dst, src, field) => igetters_display("iget-short", f, *dst, *src, *field),
            Instruction::IPut(dst, src, field) => igetters_display("iput", f, *dst, *src, *field),
            Instruction::IPutWide(dst, src, field) => igetters_display("iput-wide", f, *dst, *src, *field),
            Instruction::IPutObject(dst, src, field) => igetters_display("iput-object", f, *dst, *src, *field),
            Instruction::IPutBoolean(dst, src, field) => igetters_display("iput-boolean", f, *dst, *src, *field),
            Instruction::IPutByte(dst, src, field) => igetters_display("iput-byte", f, *dst, *src, *field),
            Instruction::IPutChar(dst, src, field) => igetters_display("iput-char", f, *dst, *src, *field),
            Instruction::IPutShort(dst, src, field) => igetters_display("iput-short", f, *dst, *src, *field),
            Instruction::SGet(dst, field) => sgetters_display("sget", f, *dst, *field),
            Instruction::SGetWide(dst, field) => sgetters_display("sget-wide", f, *dst, *field),
            Instruction::SGetObject(dst, field) => sgetters_display("sget-object", f, *dst, *field),
            Instruction::SGetBoolean(dst, field) => sgetters_display("sget-boolean", f, *dst, *field),
            Instruction::SGetByte(dst, field) => sgetters_display("sget-byte", f, *dst, *field),
            Instruction::SGetChar(dst, field) => sgetters_display("sget-char", f, *dst, *field),
            Instruction::SGetShort(dst, field) => sgetters_display("sget-short", f, *dst, *field),
            Instruction::SPut(dst, field) => sgetters_display("sput", f, *dst, *field),
            Instruction::SPutWide(dst, field) => sgetters_display("sput-wide", f, *dst, *field),
            Instruction::SPutObject(dst, field) => sgetters_display("sput-object", f, *dst, *field),
            Instruction::SPutBoolean(dst, field) => sgetters_display("sput-boolean", f, *dst, *field),
            Instruction::SPutByte(dst, field) => sgetters_display("sput-byte", f, *dst, *field),
            Instruction::SPutChar(dst, field) => sgetters_display("sput-char", f, *dst, *field),
            Instruction::SPutShort(dst, field) => sgetters_display("sput-short", f, *dst, *field),
            Instruction::InvokeVirtual { method, nargs, args } => invoke_display(f, args, nargs, *method, "virtual"),
            Instruction::InvokeSuper { method, nargs, args } => invoke_display(f, args, nargs, *method, "super"),
            Instruction::InvokeDirect { method, nargs, args } => invoke_display(f, args, nargs, *method, "direct"),
            Instruction::InvokeStatic { method, nargs, args } => invoke_display(f, args, nargs, *method, "static"),
            Instruction::InvokeInterface { method, nargs, args } => invoke_display(f, args, nargs, *method, "interface"),
            Instruction::InvokeVirtualRange { method, args } => invoke_range_display(f, args, *method, "virtual"),
            Instruction::InvokeSuperRange { method, args } => invoke_range_display(f, args, *method, "super"),
            Instruction::InvokeDirectRange { method, args } => invoke_range_display(f, args, *method, "direct"),
            Instruction::InvokeStaticRange { method, args } => invoke_range_display(f, args, *method, "static"),
            Instruction::InvokeInterfaceRange { method, args } => invoke_range_display(f, args, *method, "interface"),
            Instruction::NegInt(dst, src) => two_regs_display(f, "neg-int", *dst, *src),
            Instruction::NotInt(dst, src) => two_regs_display(f, "not-int", *dst, *src),
            Instruction::NegLong(dst, src) => two_regs_display(f, "neg-long", *dst, *src),
            Instruction::NotLong(dst, src) => two_regs_display(f, "not-long", *dst, *src),
            Instruction::NegFloat(dst, src) => two_regs_display(f, "neg-float", *dst, *src),
            Instruction::NegDouble(dst, src) => two_regs_display(f, "neg-double", *dst, *src),
            Instruction::IntToLong(dst, src) => two_regs_display(f, "int-to-long", *dst, *src),
            Instruction::IntToFloat(dst, src) => two_regs_display(f, "int-to-float", *dst, *src),
            Instruction::IntToDouble(dst, src) => two_regs_display(f, "int-to-double", *dst, *src),
            Instruction::LongToInt(dst, src) => two_regs_display(f, "long-to-int", *dst, *src),
            Instruction::LongToFloat(dst, src) => two_regs_display(f, "long-to-float", *dst, *src),
            Instruction::LongToDouble(dst, src) => two_regs_display(f, "long-to-double", *dst, *src),
            Instruction::FloatToInt(dst, src) => two_regs_display(f, "float-to-int", *dst, *src),
            Instruction::FloatToLong(dst, src) => two_regs_display(f, "float-to-long", *dst, *src),
            Instruction::FloatToDouble(dst, src) => two_regs_display(f, "float-to-double", *dst, *src),
            Instruction::DoubleToInt(dst, src) => two_regs_display(f, "double-to-int", *dst, *src),
            Instruction::DoubleToLong(dst, src) => two_regs_display(f, "double-to-long", *dst, *src),
            Instruction::DoubleToFloat(dst, src) => two_regs_display(f, "double-to-float", *dst, *src),
            Instruction::IntTobyte(dst, src) => two_regs_display(f, "int-to-byte", *dst, *src),
            Instruction::IntTochar(dst, src) => two_regs_display(f, "int-to-char", *dst, *src),
            Instruction::IntToshort(dst, src) => two_regs_display(f, "int-to-short", *dst, *src),
            Instruction::AddInt(dst, src1, src2) => three_regs_display(f, "add-int", *dst, *src1, *src2),
            Instruction::SubInt(dst, src1, src2) => three_regs_display(f, "sub-int", *dst, *src1, *src2),
            Instruction::MulInt(dst, src1, src2) => three_regs_display(f, "mul-int", *dst, *src1, *src2),
            Instruction::DivInt(dst, src1, src2) => three_regs_display(f, "div-int", *dst, *src1, *src2),
            Instruction::RemInt(dst, src1, src2) => three_regs_display(f, "rem-int", *dst, *src1, *src2),
            Instruction::AndInt(dst, src1, src2) => three_regs_display(f, "and-int", *dst, *src1, *src2),
            Instruction::OrInt(dst, src1, src2) => three_regs_display(f, "or-int", *dst, *src1, *src2),
            Instruction::XorInt(dst, src1, src2) => three_regs_display(f, "xor-int", *dst, *src1, *src2),
            Instruction::ShlInt(dst, src1, src2) => three_regs_display(f, "shl-int", *dst, *src1, *src2),
            Instruction::ShrInt(dst, src1, src2) => three_regs_display(f, "shr-int", *dst, *src1, *src2),
            Instruction::UshrInt(dst, src1, src2) => three_regs_display(f, "ushr-int", *dst, *src1, *src2),
            Instruction::AddLong(dst, src1, src2) => three_regs_display(f, "add-long", *dst, *src1, *src2),
            Instruction::SubLong(dst, src1, src2) => three_regs_display(f, "sub-long", *dst, *src1, *src2),
            Instruction::MulLong(dst, src1, src2) => three_regs_display(f, "mul-long", *dst, *src1, *src2),
            Instruction::DivLong(dst, src1, src2) => three_regs_display(f, "div-long", *dst, *src1, *src2),
            Instruction::RemLong(dst, src1, src2) => three_regs_display(f, "rem-long", *dst, *src1, *src2),
            Instruction::AndLong(dst, src1, src2) => three_regs_display(f, "and-long", *dst, *src1, *src2),
            Instruction::OrLong(dst, src1, src2) => three_regs_display(f, "or-long", *dst, *src1, *src2),
            Instruction::XorLong(dst, src1, src2) => three_regs_display(f, "xor-long", *dst, *src1, *src2),
            Instruction::ShlLong(dst, src1, src2) => three_regs_display(f, "shl-long", *dst, *src1, *src2),
            Instruction::ShrLong(dst, src1, src2) => three_regs_display(f, "shr-long", *dst, *src1, *src2),
            Instruction::UshrLong(dst, src1, src2) => three_regs_display(f, "ushr-long", *dst, *src1, *src2),
            Instruction::AddFloat(dst, src1, src2) => three_regs_display(f, "add-float", *dst, *src1, *src2),
            Instruction::SubFloat(dst, src1, src2) => three_regs_display(f, "sub-float", *dst, *src1, *src2),
            Instruction::MulFloat(dst, src1, src2) => three_regs_display(f, "mul-float", *dst, *src1, *src2),
            Instruction::DivFloat(dst, src1, src2) => three_regs_display(f, "div-float", *dst, *src1, *src2),
            Instruction::RemFloat(dst, src1, src2) => three_regs_display(f, "rem-float", *dst, *src1, *src2),
            Instruction::AddDouble(dst, src1, src2) => three_regs_display(f, "add-double", *dst, *src1, *src2),
            Instruction::SubDouble(dst, src1, src2) => three_regs_display(f, "sub-double", *dst, *src1, *src2),
            Instruction::MulDouble(dst, src1, src2) => three_regs_display(f, "mul-double", *dst, *src1, *src2),
            Instruction::DivDouble(dst, src1, src2) => three_regs_display(f, "div-double", *dst, *src1, *src2),
            Instruction::RemDouble(dst, src1, src2) => three_regs_display(f, "rem-double", *dst, *src1, *src2),
            Instruction::AddInt2(dst, src) => two_regs_display(f, "add-int/2addr", *dst, *src),
            Instruction::SubInt2(dst, src) => two_regs_display(f, "sub-int/2addr", *dst, *src),
            Instruction::MulInt2(dst, src) => two_regs_display(f, "mul-int/2addr", *dst, *src),
            Instruction::DivInt2(dst, src) => two_regs_display(f, "div-int/2addr", *dst, *src),
            Instruction::RemInt2(dst, src) => two_regs_display(f, "rem-int/2addr", *dst, *src),
            Instruction::AndInt2(dst, src) => two_regs_display(f, "and-int/2addr", *dst, *src),
            Instruction::OrInt2(dst, src) => two_regs_display(f, "or-int/2addr", *dst, *src),
            Instruction::XorInt2(dst, src) => two_regs_display(f, "xor-int/2addr", *dst, *src),
            Instruction::ShlInt2(dst, src) => two_regs_display(f, "shl-int/2addr", *dst, *src),
            Instruction::ShrInt2(dst, src) => two_regs_display(f, "shr-int/2addr", *dst, *src),
            Instruction::UShrInt2(dst, src) => two_regs_display(f, "ushr-int/2addr", *dst, *src),
            Instruction::AddLong2(dst, src) => two_regs_display(f, "add-long/2addr", *dst, *src),
            Instruction::SubLong2(dst, src) => two_regs_display(f, "sub-long/2addr", *dst, *src),
            Instruction::MulLong2(dst, src) => two_regs_display(f, "mul-long/2addr", *dst, *src),
            Instruction::DivLong2(dst, src) => two_regs_display(f, "div-long/2addr", *dst, *src),
            Instruction::RemLong2(dst, src) => two_regs_display(f, "rem-long/2addr", *dst, *src),
            Instruction::AndLong2(dst, src) => two_regs_display(f, "and-long/2addr", *dst, *src),
            Instruction::OrLong2(dst, src) => two_regs_display(f, "or-long/2addr", *dst, *src),
            Instruction::XorLong2(dst, src) => two_regs_display(f, "xor-long/2addr", *dst, *src),
            Instruction::ShlLong2(dst, src) => two_regs_display(f, "shl-long/2addr", *dst, *src),
            Instruction::ShrLong2(dst, src) => two_regs_display(f, "shr-long/2addr", *dst, *src),
            Instruction::UShrLong2(dst, src) => two_regs_display(f, "ushr-long/2addr", *dst, *src),
            Instruction::AddFloat2(dst, src) => two_regs_display(f, "add-float/2addr", *dst, *src),
            Instruction::SubFloat2(dst, src) => two_regs_display(f, "sub-float/2addr", *dst, *src),
            Instruction::MulFloat2(dst, src) => two_regs_display(f, "mul-float/2addr", *dst, *src),
            Instruction::DivFloat2(dst, src) => two_regs_display(f, "div-float/2addr", *dst, *src),
            Instruction::RemFloat2(dst, src) => two_regs_display(f, "rem-float/2addr", *dst, *src),
            Instruction::AddDouble2(dst, src) => two_regs_display(f, "add-double/2addr", *dst, *src),
            Instruction::SubDouble2(dst, src) => two_regs_display(f, "sub-double/2addr", *dst, *src),
            Instruction::MulDouble2(dst, src) => two_regs_display(f, "mul-double/2addr", *dst, *src),
            Instruction::DivDouble2(dst, src) => two_regs_display(f, "div-double/2addr", *dst, *src),
            Instruction::RemDouble2(dst, src) => two_regs_display(f, "rem-double/2addr", *dst, *src),
            Instruction::AddInt16(dst, src, lit) => f.write_fmt(format_args!("add-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::RsubInt16(dst, src, lit) => f.write_fmt(format_args!("rsub-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::MulInt16(dst, src, lit) => f.write_fmt(format_args!("mul-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::DivInt16(dst, src, lit) => f.write_fmt(format_args!("div-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::RemInt16(dst, src, lit) => f.write_fmt(format_args!("rem-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::AndInt16(dst, src, lit) => f.write_fmt(format_args!("and-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::OrInt16(dst, src, lit) => f.write_fmt(format_args!("or-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::XorInt16(dst, src, lit) => f.write_fmt(format_args!("xor-int/lit16 v{dst}, v{src}, {lit:#x}")),
            Instruction::AddInt8(dst, src, lit) => f.write_fmt(format_args!("add-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::RsubInt8(dst, src, lit) => f.write_fmt(format_args!("rsub-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::MulInt8(dst, src, lit) => f.write_fmt(format_args!("mul-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::DivInt8(dst, src, lit) => f.write_fmt(format_args!("div-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::RemInt8(dst, src, lit) => f.write_fmt(format_args!("rem-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::AndInt8(dst, src, lit) => f.write_fmt(format_args!("and-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::OrInt8(dst, src, lit) => f.write_fmt(format_args!("or-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::XorInt8(dst, src, lit) => f.write_fmt(format_args!("xor-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::ShlInt8(dst, src, lit) => f.write_fmt(format_args!("shl-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::ShrInt8(dst, src, lit) => f.write_fmt(format_args!("shr-int/lit8 v{dst}, v{src}, {lit:#x}")),
            Instruction::UshrInt8(dst, src, lit) => f.write_fmt(format_args!("ushr-int/lit8 v{dst}, v{src}, {lit:#x}")),
        }
    }
}

fn one_regs_display(f: &mut std::fmt::Formatter<'_>, verb: &str, reg: impl Into<u16>) -> Result<(), std::fmt::Error> {
    let reg = reg.into();
    f.write_fmt(format_args!("{verb} v{reg}"))
}

fn two_regs_display(f: &mut std::fmt::Formatter<'_>, verb: &str, dst: impl Into<u16>, src: impl Into<u16>) -> Result<(), std::fmt::Error> {
    let dst = dst.into();
    let src = src.into();
    f.write_fmt(format_args!("{verb} v{dst}, v{src}"))
}

fn three_regs_display(
    f: &mut std::fmt::Formatter<'_>,
    verb: &str,
    dst: impl Into<u16>,
    src1: impl Into<u16>,
    src2: impl Into<u16>,
) -> Result<(), std::fmt::Error> {
    let dst = dst.into();
    let src1 = src1.into();
    let src2 = src2.into();
    f.write_fmt(format_args!("{verb} v{dst}, v{src1}, v{src2}"))
}

fn igetters_display(verb: &str, f: &mut std::fmt::Formatter<'_>, dst: u8, src: u8, field: u16) -> Result<(), std::fmt::Error> {
    f.write_fmt(format_args!("{verb} v{dst}, v{src}, field@{field:x}"))
}

fn sgetters_display(verb: &str, f: &mut std::fmt::Formatter<'_>, dst: u8, field: u16) -> Result<(), std::fmt::Error> {
    f.write_fmt(format_args!("{verb} v{dst}, field@{field:x}"))
}

fn invoke_display(f: &mut std::fmt::Formatter<'_>, args: &[u8; 5], nargs: &u8, method: u16, kind: &'static str) -> Result<(), std::fmt::Error> {
    f.write_fmt(format_args!("invoke-{kind} {{"))?;
    for (n, arg) in args[..*nargs as usize].iter().enumerate() {
        match n {
            0 => f.write_fmt(format_args!("v{arg}"))?,
            _ => f.write_fmt(format_args!(", v{arg}"))?,
        }
    }
    f.write_fmt(format_args!("}}, method@{method:x}"))
}

fn invoke_range_display(f: &mut std::fmt::Formatter<'_>, args: &[u16], method: u16, kind: &'static str) -> Result<(), std::fmt::Error> {
    f.write_fmt(format_args!("invoke-{kind}/range {{"))?;
    for (n, arg) in args.iter().enumerate() {
        match n {
            0 => f.write_fmt(format_args!("v{arg}"))?,
            _ => f.write_fmt(format_args!(", v{arg}"))?,
        }
    }
    f.write_fmt(format_args!("}}, method@{method:x}"))
}

/// Trait for pretty printing dalvik instructions such that they include method
/// names, string literals, field names, etc.
///
/// The metadata required is usually in Dex metadata, for which this crate does
/// not parse yet.
pub trait PrettyPrint {
    /// Method lookup. Should return (Class, Name, Params, Return)
    fn method(&self, index: u16) -> (String, String, String, String);
    /// Field lookup. Should return (Class, Name, Type)
    fn field(&self, index: u16) -> (String, String, String);
    /// String lookup. Should return contents of the string at the given index.
    fn string(&self, index: u32) -> String;
    /// Type lookup. Should return the encoded name of the type.
    fn type_name(&self, index: u16) -> String;

    /// Pretty print the instruction
    ///
    /// Newline is not added to the end.
    fn print(&self, inst: &Instruction) -> String {
        match inst {
            Instruction::ConstString(dst, idx) => format!("const-string v{dst}, \"{}\"", self.string((*idx).into())),
            Instruction::ConstStringJumbo(dst, idx) => format!("const-string/jumbo v{dst}, \"{}\"", self.string(*idx)),
            Instruction::ConstClass(dst, idx) => format!("const-class v{dst}, {}", self.type_name(*idx)),
            Instruction::NewInstance(reg, ty) => format!("new-instance v{reg}, {}", self.type_name(*ty)),
            Instruction::NewArray(dst, size, ty) => format!("new-array v{dst}, v{size}, {}", self.type_name(*ty)),
            Instruction::FilledNewArray { ty, nargs, args } => {
                let ty = self.type_name(*ty);

                let mut s = format!("filled-new-array {{");
                for (n, arg) in args[..*nargs as usize].iter().enumerate() {
                    match n {
                        0 => s.push_str(&format!("v{arg}")),
                        _ => s.push_str(&format!(", v{arg}")),
                    }
                }
                s.push_str(&format!("}}, {ty}"));
                s
            }
            Instruction::FilledNewArrayRange { ty, args } => {
                let ty = self.type_name(*ty);

                let mut s = format!("filled-new-array/range {{");
                for (n, arg) in args.iter().enumerate() {
                    match n {
                        0 => s.push_str(&format!("v{arg}")),
                        _ => s.push_str(&format!(", v{arg}")),
                    }
                }
                s.push_str(&format!("}}, {ty}"));
                s
            }
            Instruction::IGet(dst, src, field) => render_isgetters(self, "iget", *dst, Some(*src), *field),
            Instruction::IGetWide(dst, src, field) => render_isgetters(self, "iget-wide", *dst, Some(*src), *field),
            Instruction::IGetObject(dst, src, field) => render_isgetters(self, "iget-object", *dst, Some(*src), *field),
            Instruction::IGetBoolean(dst, src, field) => render_isgetters(self, "iget-boolean", *dst, Some(*src), *field),
            Instruction::IGetByte(dst, src, field) => render_isgetters(self, "iget-byte", *dst, Some(*src), *field),
            Instruction::IGetChar(dst, src, field) => render_isgetters(self, "iget-char", *dst, Some(*src), *field),
            Instruction::IGetShort(dst, src, field) => render_isgetters(self, "iget-short", *dst, Some(*src), *field),
            Instruction::IPut(dst, src, field) => render_isgetters(self, "iput", *dst, Some(*src), *field),
            Instruction::IPutWide(dst, src, field) => render_isgetters(self, "iput-wide", *dst, Some(*src), *field),
            Instruction::IPutObject(dst, src, field) => render_isgetters(self, "iput-object", *dst, Some(*src), *field),
            Instruction::IPutBoolean(dst, src, field) => render_isgetters(self, "iput-boolean", *dst, Some(*src), *field),
            Instruction::IPutByte(dst, src, field) => render_isgetters(self, "iput-byte", *dst, Some(*src), *field),
            Instruction::IPutChar(dst, src, field) => render_isgetters(self, "iput-char", *dst, Some(*src), *field),
            Instruction::IPutShort(dst, src, field) => render_isgetters(self, "iput-short", *dst, Some(*src), *field),
            Instruction::SGet(dst, field) => render_isgetters(self, "sget", *dst, None, *field),
            Instruction::SGetWide(dst, field) => render_isgetters(self, "sget-wide", *dst, None, *field),
            Instruction::SGetObject(dst, field) => render_isgetters(self, "sget-object", *dst, None, *field),
            Instruction::SGetBoolean(dst, field) => render_isgetters(self, "sget-boolean", *dst, None, *field),
            Instruction::SGetByte(dst, field) => render_isgetters(self, "sget-byte", *dst, None, *field),
            Instruction::SGetChar(dst, field) => render_isgetters(self, "sget-char", *dst, None, *field),
            Instruction::SGetShort(dst, field) => render_isgetters(self, "sget-short", *dst, None, *field),
            Instruction::SPut(dst, field) => render_isgetters(self, "sput", *dst, None, *field),
            Instruction::SPutWide(dst, field) => render_isgetters(self, "sput-wide", *dst, None, *field),
            Instruction::SPutObject(dst, field) => render_isgetters(self, "sput-object", *dst, None, *field),
            Instruction::SPutBoolean(dst, field) => render_isgetters(self, "sput-boolean", *dst, None, *field),
            Instruction::SPutByte(dst, field) => render_isgetters(self, "sput-byte", *dst, None, *field),
            Instruction::SPutChar(dst, field) => render_isgetters(self, "sput-char", *dst, None, *field),
            Instruction::SPutShort(dst, field) => render_isgetters(self, "sput-short", *dst, None, *field),
            Instruction::CheckCast(reg, ty) => format!("check-cast v{reg}, {}", self.type_name(*ty)),
            Instruction::InstanceOf(dst, src, ty) => format!("instance-of v{dst}, v{src}, {}", self.type_name(*ty)),
            Instruction::InvokeVirtual { method, nargs, args } => render_invoke(self, *method, args, *nargs, "virtual"),
            Instruction::InvokeSuper { method, nargs, args } => render_invoke(self, *method, args, *nargs, "super"),
            Instruction::InvokeStatic { method, nargs, args } => render_invoke(self, *method, args, *nargs, "static"),
            Instruction::InvokeDirect { method, nargs, args } => render_invoke(self, *method, args, *nargs, "direct"),
            Instruction::InvokeInterface { method, nargs, args } => render_invoke(self, *method, args, *nargs, "interface"),
            Instruction::InvokeVirtualRange { method, args } => render_invoke_range(self, *method, args, "virtual"),
            Instruction::InvokeSuperRange { method, args } => render_invoke_range(self, *method, args, "super"),
            Instruction::InvokeStaticRange { method, args } => render_invoke_range(self, *method, args, "static"),
            Instruction::InvokeDirectRange { method, args } => render_invoke_range(self, *method, args, "direct"),
            Instruction::InvokeInterfaceRange { method, args } => render_invoke_range(self, *method, args, "interface"),
            no_lookup => no_lookup.to_string(),
        }
    }
}

fn render_isgetters<T: PrettyPrint + ?Sized>(lookup: &T, verb: &str, dst: u8, src: Option<u8>, field: u16) -> String {
    let (class, name, ty) = lookup.field(field);
    let mut s = format!("{verb} v{dst}, ");
    if let Some(src) = src {
        s.push_str(&format!("{src}, "));
    }
    s.push_str(&format!("{class}->{name}:{ty}"));
    s
}

fn render_invoke<T: PrettyPrint + ?Sized>(lookup: &T, method: u16, args: &[u8; 5], nargs: u8, kind: &'static str) -> String {
    let (class, name, params, ret) = lookup.method(method);

    let mut s = format!("invoke-{kind} {{");
    for (n, arg) in args[..nargs as usize].iter().enumerate() {
        match n {
            0 => s.push_str(&format!("v{arg}")),
            _ => s.push_str(&format!(", v{arg}")),
        }
    }
    s.push_str(&format!("}}, {class}->{name}({params}){ret}"));
    s
}
fn render_invoke_range<T: PrettyPrint + ?Sized>(lookup: &T, method: u16, args: &[u16], kind: &'static str) -> String {
    let (class, name, params, ret) = lookup.method(method);

    let mut s = format!("invoke-{kind}/range {{");
    for (n, arg) in args.iter().enumerate() {
        match n {
            0 => s.push_str(&format!("v{arg}")),
            _ => s.push_str(&format!(", v{arg}")),
        }
    }
    s.push_str(&format!("}}, {class}->{name}({params}){ret}"));
    s
}
