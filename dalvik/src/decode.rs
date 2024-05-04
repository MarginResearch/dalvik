//! Dalvik bytecode instruction decoding

use crate::Instruction;

/// Decoding error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An instruction was truncated
    ///
    /// More codepoints are needed to decode properly
    Truncated,
    /// Instruction was not encoded correctly
    Encoding,
    /// The bytecode was inline metadata and should be skipped over
    ///
    /// Possible tables formats:
    ///  - [Packed Switch Payload][1]
    ///  - [Sparse Switch Payload][2]
    ///  - [Fill Array Data Payload][3]
    ///
    /// [1]: https://source.android.com/docs/core/runtime/dalvik-bytecode#packed-switch
    /// [2]: https://source.android.com/docs/core/runtime/dalvik-bytecode#sparse-switch
    /// [3]: https://source.android.com/docs/core/runtime/dalvik-bytecode#fill-array
    Metadata {
        /// Length (in u16 codepoints) of the table
        length: usize,
    },
}

/// Decode all [`Instructions`][`Instruction`] from a slice of codepoints
pub fn decode_all(mut bytecode: &[u16], until: usize) -> Result<Vec<Instruction>, Error> {
    let mut ins = Vec::new();
    while !bytecode.is_empty() && ins.len() < until {
        dbg!(bytecode.len());
        ins.push(match decode_one(&mut bytecode) {
            Ok(i) => i,
            // skip over metadata tables
            Err(Error::Metadata { length }) => {
                bytecode = &bytecode[length..];
                continue;
            }
            Err(e) => return Err(e),
        });
    }
    Ok(ins)
}

/// Decode one [`Instruction`], advancing the given slice to the next instruction
pub fn decode_one(bytecode: &mut &[u16]) -> Result<Instruction, Error> {
    let op = bytecode[0] as u8;
    let inst = match op {
        opcode::NOP => match d::aa_op(bytecode)? {
            0x00 => Instruction::Nop,
            // packed-switch-payload
            // ident    ushort  opcode, already parsed
            // size      ushort number of entries in the table
            // first_key int    first (and lowest) switch case value
            // targets   int[]  list of `size` relative branch targets
            0x01 => {
                let size = d::consume_u16(bytecode)?;
                let _first_key = d::consume_u32(bytecode)?;
                // skip the targets table
                let num_codes = 2                // 2 u16 per u32
                                * size as usize; // length of each table;
                if bytecode.len() < num_codes {
                    return Err(Error::Truncated);
                }
                todo!("handle inline metadata?");
            }
            // sparse-switch-payload
            // ident    ushort  opcode, already parsed
            // size     ushort  number of entries in the table
            // keys     int[]   list of `size` key values
            // targets  int[]   list of `size` relative branch targets
            0x02 => {
                let size = d::consume_u16(bytecode)?;
                // skip the keys and targets tables
                let num_codes = 2                // 2 u16 per u32
                                * 2              // 2 tables of equal length
                                * size as usize; // length of each table;
                if bytecode.len() < num_codes {
                    return Err(Error::Truncated);
                }
                todo!("handle inline metadata?");
            }
            // fill-array-data-payload
            // element_width  ushort   number of bytes in each element
            // size           uint     number of elements in the table
            // data           ubyte[]  data values
            //
            // NOTE: The total number of code units for an instance of this
            // table is (size * element_width + 1) / 2 + 6
            // ERRATA: The note in the reference miscalculates the size
            0x03 => {
                let element_width = d::consume_u16(bytecode)?;
                let size = d::consume_u32(bytecode)?;
                let code_size = (element_width as usize * size as usize + 1) / 2;
                if bytecode.len() < code_size {
                    return Err(Error::Truncated);
                }
                todo!("handle inline metadata?");
            }
            _ => return Err(Error::Encoding),
        },
        opcode::MOVE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::Move(dst, src)
        }
        opcode::MOVEFROM16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::MoveFrom16(dst, src)
        }
        opcode::MOVE16 => {
            let (dst, src) = d::zz_op_aaaabbbb(bytecode)?;
            Instruction::Move16(dst, src)
        }
        opcode::MOVEWIDE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MoveWide(dst, src)
        }
        opcode::MOVEWIDEFROM16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::MoveWideFrom16(dst, src)
        }
        opcode::MOVEWIDE16 => {
            let (dst, src) = d::zz_op_aaaabbbb(bytecode)?;
            Instruction::MoveWide16(dst, src)
        }
        opcode::MOVEOBJECT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MoveObject(dst, src)
        }
        opcode::MOVEOBJECTFROM16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::MoveObjectFrom16(dst, src)
        }
        opcode::MOVEOBJECT16 => {
            let (dst, src) = d::zz_op_aaaabbbb(bytecode)?;
            Instruction::MoveObject16(dst, src)
        }
        opcode::MOVERESULT => {
            let dst = d::aa_op(bytecode)?;
            Instruction::MoveResult(dst)
        }
        opcode::MOVERESULTWIDE => {
            let dst = d::aa_op(bytecode)?;
            Instruction::MoveResultWide(dst)
        }
        opcode::MOVERESULTOBJECT => {
            let dst = d::aa_op(bytecode)?;
            Instruction::MoveResultObject(dst)
        }
        opcode::MOVEEXCEPTION => {
            let dst = d::aa_op(bytecode)?;
            Instruction::MoveException(dst)
        }
        opcode::RETURNVOID => {
            d::zz_op(bytecode)?;
            Instruction::ReturnVoid
        }
        opcode::RETURN => {
            let reg = d::aa_op(bytecode)?;
            Instruction::Return(reg)
        }
        opcode::RETURNWIDE => {
            let reg = d::aa_op(bytecode)?;
            Instruction::ReturnWide(reg)
        }
        opcode::RETURNOBJECT => {
            let reg = d::aa_op(bytecode)?;
            Instruction::ReturnObject(reg)
        }
        opcode::CONST4 => {
            let (mut src, dst) = d::ba_op(bytecode)?;
            // sign extend the 4-bit literal to i8
            if src & 0b1000 > 0 {
                src = src | 0xf0;
            }
            Instruction::Const4(dst, src as i8)
        }
        opcode::CONST16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::Const16(dst, src as i16)
        }
        opcode::CONST => {
            let (dst, src) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::Const(dst, src)
        }
        opcode::CONSTHIGH16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::ConstHigh16(dst, src as i16)
        }
        opcode::CONSTWIDE16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::ConstWide16(dst, src as i16)
        }
        opcode::CONSTWIDE32 => {
            let (dst, src) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::ConstWide32(dst, src)
        }
        opcode::CONSTWIDE => {
            let (dst, src) = d::aa_op_bbbbbbbbbbbbbbbb(bytecode)?;
            Instruction::ConstWide(dst, src)
        }
        opcode::CONSTWIDEHIGH16 => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::ConstWideHigh16(dst, src)
        }
        opcode::CONSTSTRING => {
            let (dst, src) = d::aa_op_bbbb(bytecode)?;
            Instruction::ConstString(dst, src)
        }
        opcode::CONSTSTRINGJUMBO => {
            let (dst, src) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::ConstStringJumbo(dst, src)
        }
        opcode::CONSTCLASS => {
            let (dst, class) = d::aa_op_bbbb(bytecode)?;
            Instruction::ConstClass(dst, class)
        }
        opcode::MONITORENTER => {
            let reg = d::aa_op(bytecode)?;
            Instruction::MonitorEnter(reg)
        }
        opcode::MONITOREXIT => {
            let reg = d::aa_op(bytecode)?;
            Instruction::MonitorExit(reg)
        }
        opcode::CHECKCAST => {
            let (reg, ty) = d::aa_op_bbbb(bytecode)?;
            Instruction::CheckCast(reg, ty)
        }
        opcode::INSTANCEOF => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::InstanceOf(dst, src, ty)
        }
        opcode::ARRAYLENGTH => {
            let (dst, src) = d::ba_op(bytecode)?;
            Instruction::ArrayLength(dst, src)
        }
        opcode::NEWINSTANCE => {
            let (reg, ty) = d::aa_op_bbbb(bytecode)?;
            Instruction::NewInstance(reg, ty)
        }
        opcode::NEWARRAY => {
            let (size, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::NewArray(dst, size, ty)
        }
        opcode::FILLEDNEWARRAY => {
            let (nargs, g, ty, f, e, d, c) = d::ag_op_bbbbfedc(bytecode)?;
            let args = [c, d, e, f, g];
            Instruction::FilledNewArray { ty, nargs, args }
        }
        opcode::FILLEDNEWARRAYRANGE => {
            let (count, ty, start) = d::aa_op_ccccbbbb(bytecode)?;
            let mut args = Vec::with_capacity(count as usize);
            for r in start..start + count as u16 {
                args.push(r);
            }
            Instruction::FilledNewArrayRange { ty, args }
        }
        opcode::FILLARRAYDATA => {
            let (dst, table) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::FillArrayData(dst, table as i32)
        }
        opcode::THROW => {
            let reg = d::aa_op(bytecode)?;
            Instruction::Throw(reg)
        }
        opcode::GOTO => {
            let dst = d::aa_op(bytecode)?;
            Instruction::Goto(dst as i8)
        }
        opcode::GOTO16 => {
            let dst = d::zz_op_aaaa(bytecode)?;
            Instruction::Goto16(dst as i16)
        }
        opcode::GOTO32 => {
            let dst = d::zz_op_aaaaaaaa(bytecode)?;
            Instruction::Goto32(dst as i32)
        }
        opcode::PACKEDSWITCH => {
            let (reg, table) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::PackedSwitch(reg, table as i32)
        }
        opcode::SPARSESWITCH => {
            let (reg, table) = d::aa_op_bbbbbbbb(bytecode)?;
            Instruction::SparseSwitch(reg, table as i32)
        }
        opcode::CMPLFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::CmplFloat(dst, src1, src2)
        }
        opcode::CMPGFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::CmpgFloat(dst, src1, src2)
        }
        opcode::CMPLDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::CmplDouble(dst, src1, src2)
        }
        opcode::CMPGDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::CmpgDouble(dst, src1, src2)
        }
        opcode::CMPLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::CmpLong(dst, src1, src2)
        }
        opcode::IFEQ => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfEq(a, b, off as i16)
        }
        opcode::IFNE => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfNe(a, b, off as i16)
        }
        opcode::IFLT => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfLt(a, b, off as i16)
        }
        opcode::IFGE => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfGe(a, b, off as i16)
        }
        opcode::IFGT => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfGt(a, b, off as i16)
        }
        opcode::IFLE => {
            let (b, a, off) = d::ba_op_cccc(bytecode)?;
            Instruction::IfLe(a, b, off as i16)
        }
        opcode::IFEQZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfEqz(reg, off as i16)
        }
        opcode::IFNEZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfNez(reg, off as i16)
        }
        opcode::IFLTZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfLtz(reg, off as i16)
        }
        opcode::IFGEZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfGez(reg, off as i16)
        }
        opcode::IFGTZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfGtz(reg, off as i16)
        }
        opcode::IFLEZ => {
            let (reg, off) = d::aa_op_bbbb(bytecode)?;
            Instruction::IfLez(reg, off as i16)
        }
        opcode::AGET => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGet(dst, src1, src2)
        }
        opcode::AGETWIDE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetWide(dst, src1, src2)
        }
        opcode::AGETOBJECT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetObject(dst, src1, src2)
        }
        opcode::AGETBOOLEAN => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetBoolean(dst, src1, src2)
        }
        opcode::AGETBYTE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetByte(dst, src1, src2)
        }
        opcode::AGETCHAR => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetChar(dst, src1, src2)
        }
        opcode::AGETSHORT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AGetShort(dst, src1, src2)
        }
        opcode::APUT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APut(dst, src1, src2)
        }
        opcode::APUTWIDE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutWide(dst, src1, src2)
        }
        opcode::APUTOBJECT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutObject(dst, src1, src2)
        }
        opcode::APUTBOOLEAN => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutBoolean(dst, src1, src2)
        }
        opcode::APUTBYTE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutByte(dst, src1, src2)
        }
        opcode::APUTCHAR => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutChar(dst, src1, src2)
        }
        opcode::APUTSHORT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::APutShort(dst, src1, src2)
        }
        opcode::IGET => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGet(dst, src, ty)
        }
        opcode::IGETWIDE => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetWide(dst, src, ty)
        }
        opcode::IGETOBJECT => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetObject(dst, src, ty)
        }
        opcode::IGETBOOLEAN => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetBoolean(dst, src, ty)
        }
        opcode::IGETBYTE => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetByte(dst, src, ty)
        }
        opcode::IGETCHAR => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetChar(dst, src, ty)
        }
        opcode::IGETSHORT => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IGetShort(dst, src, ty)
        }
        opcode::IPUT => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPut(dst, src, ty)
        }
        opcode::IPUTWIDE => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutWide(dst, src, ty)
        }
        opcode::IPUTOBJECT => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutObject(dst, src, ty)
        }
        opcode::IPUTBOOLEAN => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutBoolean(dst, src, ty)
        }
        opcode::IPUTBYTE => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutByte(dst, src, ty)
        }
        opcode::IPUTCHAR => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutChar(dst, src, ty)
        }
        opcode::IPUTSHORT => {
            let (src, dst, ty) = d::ba_op_cccc(bytecode)?;
            Instruction::IPutShort(dst, src, ty)
        }
        opcode::SGET => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGet(dst, field)
        }
        opcode::SGETWIDE => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetWide(dst, field)
        }
        opcode::SGETOBJECT => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetObject(dst, field)
        }
        opcode::SGETBOOLEAN => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetBoolean(dst, field)
        }
        opcode::SGETBYTE => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetByte(dst, field)
        }
        opcode::SGETCHAR => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetChar(dst, field)
        }
        opcode::SGETSHORT => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SGetShort(dst, field)
        }
        opcode::SPUT => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPut(dst, field)
        }
        opcode::SPUTWIDE => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutWide(dst, field)
        }
        opcode::SPUTOBJECT => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutObject(dst, field)
        }
        opcode::SPUTBOOLEAN => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutBoolean(dst, field)
        }
        opcode::SPUTBYTE => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutByte(dst, field)
        }
        opcode::SPUTCHAR => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutChar(dst, field)
        }
        opcode::SPUTSHORT => {
            let (dst, field) = d::aa_op_bbbb(bytecode)?;
            Instruction::SPutShort(dst, field)
        }
        opcode::INVOKEVIRTUAL | opcode::INVOKESUPER | opcode::INVOKEDIRECT | opcode::INVOKESTATIC | opcode::INVOKEINTERFACE => {
            let (nargs, g, method, f, e, d, c) = d::ag_op_bbbbfedc(bytecode)?;
            let args = [c, d, e, f, g];
            match op {
                opcode::INVOKEVIRTUAL => Instruction::InvokeVirtual { method, nargs, args },
                opcode::INVOKESUPER => Instruction::InvokeSuper { method, nargs, args },
                opcode::INVOKEDIRECT => Instruction::InvokeDirect { method, nargs, args },
                opcode::INVOKESTATIC => Instruction::InvokeStatic { method, nargs, args },
                opcode::INVOKEINTERFACE => Instruction::InvokeInterface { method, nargs, args },
                _ => unreachable!(),
            }
        }
        opcode::INVOKEVIRTUALRANGE | opcode::INVOKESUPERRANGE | opcode::INVOKEDIRECTRANGE | opcode::INVOKESTATICRANGE | opcode::INVOKEINTERFACERANGE => {
            let (count, method, start) = d::aa_op_ccccbbbb(bytecode)?;
            let mut args = Vec::with_capacity(count as usize);
            for r in start..start + count as u16 {
                args.push(r);
            }
            match op {
                opcode::INVOKEVIRTUALRANGE => Instruction::InvokeVirtualRange { method, args },
                opcode::INVOKESUPERRANGE => Instruction::InvokeSuperRange { method, args },
                opcode::INVOKEDIRECTRANGE => Instruction::InvokeDirectRange { method, args },
                opcode::INVOKESTATICRANGE => Instruction::InvokeStaticRange { method, args },
                opcode::INVOKEINTERFACERANGE => Instruction::InvokeInterfaceRange { method, args },
                _ => unreachable!(),
            }
        }
        opcode::NEGINT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NegInt(dst, src)
        }
        opcode::NOTINT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NotInt(dst, src)
        }
        opcode::NEGLONG => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NegLong(dst, src)
        }
        opcode::NOTLONG => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NotLong(dst, src)
        }
        opcode::NEGFLOAT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NegFloat(dst, src)
        }
        opcode::NEGDOUBLE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::NegDouble(dst, src)
        }
        opcode::INTTOLONG => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntToLong(dst, src)
        }
        opcode::INTTOFLOAT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntToFloat(dst, src)
        }
        opcode::INTTODOUBLE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntToDouble(dst, src)
        }
        opcode::LONGTOINT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::LongToInt(dst, src)
        }
        opcode::LONGTOFLOAT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::LongToFloat(dst, src)
        }
        opcode::LONGTODOUBLE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::LongToDouble(dst, src)
        }
        opcode::FLOATTOINT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::FloatToInt(dst, src)
        }
        opcode::FLOATTOLONG => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::FloatToLong(dst, src)
        }
        opcode::FLOATTODOUBLE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::FloatToDouble(dst, src)
        }
        opcode::DOUBLETOINT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DoubleToInt(dst, src)
        }
        opcode::DOUBLETOLONG => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DoubleToLong(dst, src)
        }
        opcode::DOUBLETOFLOAT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DoubleToFloat(dst, src)
        }
        opcode::INTTOBYTE => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntTobyte(dst, src)
        }
        opcode::INTTOCHAR => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntTochar(dst, src)
        }
        opcode::INTTOSHORT => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::IntToshort(dst, src)
        }
        opcode::ADDINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AddInt(dst, src1, src2)
        }
        opcode::SUBINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::SubInt(dst, src1, src2)
        }
        opcode::MULINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::MulInt(dst, src1, src2)
        }
        opcode::DIVINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::DivInt(dst, src1, src2)
        }
        opcode::REMINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::RemInt(dst, src1, src2)
        }
        opcode::ANDINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AndInt(dst, src1, src2)
        }
        opcode::ORINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::OrInt(dst, src1, src2)
        }
        opcode::XORINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::XorInt(dst, src1, src2)
        }
        opcode::SHLINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShlInt(dst, src1, src2)
        }
        opcode::SHRINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShrInt(dst, src1, src2)
        }
        opcode::USHRINT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::UshrInt(dst, src1, src2)
        }
        opcode::ADDLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AddLong(dst, src1, src2)
        }
        opcode::SUBLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::SubLong(dst, src1, src2)
        }
        opcode::MULLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::MulLong(dst, src1, src2)
        }
        opcode::DIVLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::DivLong(dst, src1, src2)
        }
        opcode::REMLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::RemLong(dst, src1, src2)
        }
        opcode::ANDLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AndLong(dst, src1, src2)
        }
        opcode::ORLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::OrLong(dst, src1, src2)
        }
        opcode::XORLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::XorLong(dst, src1, src2)
        }
        opcode::SHLLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShlLong(dst, src1, src2)
        }
        opcode::SHRLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShrLong(dst, src1, src2)
        }
        opcode::USHRLONG => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::UshrLong(dst, src1, src2)
        }
        opcode::ADDFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AddFloat(dst, src1, src2)
        }
        opcode::SUBFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::SubFloat(dst, src1, src2)
        }
        opcode::MULFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::MulFloat(dst, src1, src2)
        }
        opcode::DIVFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::DivFloat(dst, src1, src2)
        }
        opcode::REMFLOAT => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::RemFloat(dst, src1, src2)
        }
        opcode::ADDDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::AddDouble(dst, src1, src2)
        }
        opcode::SUBDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::SubDouble(dst, src1, src2)
        }
        opcode::MULDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::MulDouble(dst, src1, src2)
        }
        opcode::DIVDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::DivDouble(dst, src1, src2)
        }
        opcode::REMDOUBLE => {
            let (dst, src2, src1) = d::aa_op_ccbb(bytecode)?;
            Instruction::RemDouble(dst, src1, src2)
        }
        opcode::ADDINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AddInt2(dst, src)
        }
        opcode::SUBINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::SubInt2(dst, src)
        }
        opcode::MULINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MulInt2(dst, src)
        }
        opcode::DIVINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DivInt2(dst, src)
        }
        opcode::REMINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::RemInt2(dst, src)
        }
        opcode::ANDINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AndInt2(dst, src)
        }
        opcode::ORINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::OrInt2(dst, src)
        }
        opcode::XORINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::XorInt2(dst, src)
        }
        opcode::SHLINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::ShlInt2(dst, src)
        }
        opcode::SHRINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::ShrInt2(dst, src)
        }
        opcode::USHRINT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::UShrInt2(dst, src)
        }
        opcode::ADDLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AddLong2(dst, src)
        }
        opcode::SUBLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::SubLong2(dst, src)
        }
        opcode::MULLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MulLong2(dst, src)
        }
        opcode::DIVLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DivLong2(dst, src)
        }
        opcode::REMLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::RemLong2(dst, src)
        }
        opcode::ANDLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AndLong2(dst, src)
        }
        opcode::ORLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::OrLong2(dst, src)
        }
        opcode::XORLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::XorLong2(dst, src)
        }
        opcode::SHLLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::ShlLong2(dst, src)
        }
        opcode::SHRLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::ShrLong2(dst, src)
        }
        opcode::USHRLONG2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::UShrLong2(dst, src)
        }
        opcode::ADDFLOAT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AddFloat2(dst, src)
        }
        opcode::SUBFLOAT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::SubFloat2(dst, src)
        }
        opcode::MULFLOAT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MulFloat2(dst, src)
        }
        opcode::DIVFLOAT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DivFloat2(dst, src)
        }
        opcode::REMFLOAT2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::RemFloat2(dst, src)
        }
        opcode::ADDDOUBLE2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::AddDouble2(dst, src)
        }
        opcode::SUBDOUBLE2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::SubDouble2(dst, src)
        }
        opcode::MULDOUBLE2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::MulDouble2(dst, src)
        }
        opcode::DIVDOUBLE2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::DivDouble2(dst, src)
        }
        opcode::REMDOUBLE2 => {
            let (src, dst) = d::ba_op(bytecode)?;
            Instruction::RemDouble2(dst, src)
        }
        opcode::ADDINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::AddInt16(dst, src, lit as i16)
        }
        opcode::RSUBINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::RsubInt16(dst, src, lit as i16)
        }
        opcode::MULINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::MulInt16(dst, src, lit as i16)
        }
        opcode::DIVINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::DivInt16(dst, src, lit as i16)
        }
        opcode::REMINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::RemInt16(dst, src, lit as i16)
        }
        opcode::ANDINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::AndInt16(dst, src, lit as i16)
        }
        opcode::ORINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::OrInt16(dst, src, lit as i16)
        }
        opcode::XORINT16 => {
            let (src, dst, lit) = d::ba_op_cccc(bytecode)?;
            Instruction::XorInt16(dst, src, lit as i16)
        }
        opcode::ADDINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::AddInt8(dst, src, lit as i8)
        }
        opcode::RSUBINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::RsubInt8(dst, src, lit as i8)
        }
        opcode::MULINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::MulInt8(dst, src, lit as i8)
        }
        opcode::DIVINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::DivInt8(dst, src, lit as i8)
        }
        opcode::REMINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::RemInt8(dst, src, lit as i8)
        }
        opcode::ANDINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::AndInt8(dst, src, lit as i8)
        }
        opcode::ORINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::OrInt8(dst, src, lit as i8)
        }
        opcode::XORINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::XorInt8(dst, src, lit as i8)
        }
        opcode::SHLINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShlInt8(dst, src, lit as i8)
        }
        opcode::SHRINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::ShrInt8(dst, src, lit as i8)
        }
        opcode::USHRINT8 => {
            let (dst, lit, src) = d::aa_op_ccbb(bytecode)?;
            Instruction::UshrInt8(dst, src, lit as i8)
        }
        unk => todo!("handle opcode {unk:#x?}"),
    };

    Ok(inst)
}

impl Instruction {
    /// Length in u16 codepoints needed to encode/decode
    pub fn len(&self) -> usize {
        match self {
            Instruction::Nop => 1,
            Instruction::Move(_, _) => 1,
            Instruction::MoveFrom16(_, _) => 2,
            Instruction::Move16(_, _) => 3,
            Instruction::MoveWide(_, _) => 1,
            Instruction::MoveWideFrom16(_, _) => 2,
            Instruction::MoveWide16(_, _) => 3,
            Instruction::MoveObject(_, _) => 1,
            Instruction::MoveObjectFrom16(_, _) => 2,
            Instruction::MoveObject16(_, _) => 3,
            Instruction::MoveResult(_) => 1,
            Instruction::MoveResultWide(_) => 1,
            Instruction::MoveResultObject(_) => 1,
            Instruction::MoveException(_) => 1,
            Instruction::ReturnVoid => 1,
            Instruction::Return(_) => 1,
            Instruction::ReturnWide(_) => 1,
            Instruction::ReturnObject(_) => 1,
            Instruction::Const4(_, _) => 1,
            Instruction::Const16(_, _) => 2,
            Instruction::Const(_, _) => 3,
            Instruction::ConstHigh16(_, _) => 2,
            Instruction::ConstWide16(_, _) => 2,
            Instruction::ConstWide32(_, _) => 3,
            Instruction::ConstWide(_, _) => 5,
            Instruction::ConstWideHigh16(_, _) => 2,
            Instruction::ConstString(_, _) => 2,
            Instruction::ConstStringJumbo(_, _) => 3,
            Instruction::ConstClass(_, _) => 2,
            Instruction::MonitorEnter(_) => 1,
            Instruction::MonitorExit(_) => 1,
            Instruction::CheckCast(_, _) => 2,
            Instruction::InstanceOf(_, _, _) => 2,
            Instruction::ArrayLength(_, _) => 1,
            Instruction::NewInstance(_, _) => 2,
            Instruction::NewArray(_, _, _) => 2,
            Instruction::FilledNewArray { .. } => 3,
            Instruction::FilledNewArrayRange { .. } => 3,
            Instruction::FillArrayData(_, _) => 3,
            Instruction::Throw(_) => 1,
            Instruction::Goto(_) => 1,
            Instruction::Goto16(_) => 2,
            Instruction::Goto32(_) => 3,
            Instruction::PackedSwitch(_, _) => 3,
            Instruction::SparseSwitch(_, _) => 3,
            Instruction::CmplFloat(_, _, _) => 2,
            Instruction::CmpgFloat(_, _, _) => 2,
            Instruction::CmplDouble(_, _, _) => 2,
            Instruction::CmpgDouble(_, _, _) => 2,
            Instruction::CmpLong(_, _, _) => 2,
            Instruction::IfEq(_, _, _) => 2,
            Instruction::IfNe(_, _, _) => 2,
            Instruction::IfLt(_, _, _) => 2,
            Instruction::IfGe(_, _, _) => 2,
            Instruction::IfGt(_, _, _) => 2,
            Instruction::IfLe(_, _, _) => 2,
            Instruction::IfEqz(_, _) => 2,
            Instruction::IfNez(_, _) => 2,
            Instruction::IfLtz(_, _) => 2,
            Instruction::IfGez(_, _) => 2,
            Instruction::IfGtz(_, _) => 2,
            Instruction::IfLez(_, _) => 2,
            Instruction::AGet(_, _, _) => 2,
            Instruction::AGetWide(_, _, _) => 2,
            Instruction::AGetObject(_, _, _) => 2,
            Instruction::AGetBoolean(_, _, _) => 2,
            Instruction::AGetByte(_, _, _) => 2,
            Instruction::AGetChar(_, _, _) => 2,
            Instruction::AGetShort(_, _, _) => 2,
            Instruction::APut(_, _, _) => 2,
            Instruction::APutWide(_, _, _) => 2,
            Instruction::APutObject(_, _, _) => 2,
            Instruction::APutBoolean(_, _, _) => 2,
            Instruction::APutByte(_, _, _) => 2,
            Instruction::APutChar(_, _, _) => 2,
            Instruction::APutShort(_, _, _) => 2,
            Instruction::IGet(_, _, _) => 2,
            Instruction::IGetWide(_, _, _) => 2,
            Instruction::IGetObject(_, _, _) => 2,
            Instruction::IGetBoolean(_, _, _) => 2,
            Instruction::IGetByte(_, _, _) => 2,
            Instruction::IGetChar(_, _, _) => 2,
            Instruction::IGetShort(_, _, _) => 2,
            Instruction::IPut(_, _, _) => 2,
            Instruction::IPutWide(_, _, _) => 2,
            Instruction::IPutObject(_, _, _) => 2,
            Instruction::IPutBoolean(_, _, _) => 2,
            Instruction::IPutByte(_, _, _) => 2,
            Instruction::IPutChar(_, _, _) => 2,
            Instruction::IPutShort(_, _, _) => 2,
            Instruction::SGet(_, _) => 2,
            Instruction::SGetWide(_, _) => 2,
            Instruction::SGetObject(_, _) => 2,
            Instruction::SGetBoolean(_, _) => 2,
            Instruction::SGetByte(_, _) => 2,
            Instruction::SGetChar(_, _) => 2,
            Instruction::SGetShort(_, _) => 2,
            Instruction::SPut(_, _) => 2,
            Instruction::SPutWide(_, _) => 2,
            Instruction::SPutObject(_, _) => 2,
            Instruction::SPutBoolean(_, _) => 2,
            Instruction::SPutByte(_, _) => 2,
            Instruction::SPutChar(_, _) => 2,
            Instruction::SPutShort(_, _) => 2,
            Instruction::InvokeVirtual { .. } => 3,
            Instruction::InvokeSuper { .. } => 3,
            Instruction::InvokeDirect { .. } => 3,
            Instruction::InvokeStatic { .. } => 3,
            Instruction::InvokeInterface { .. } => 3,
            Instruction::InvokeVirtualRange { .. } => 3,
            Instruction::InvokeSuperRange { .. } => 3,
            Instruction::InvokeDirectRange { .. } => 3,
            Instruction::InvokeStaticRange { .. } => 3,
            Instruction::InvokeInterfaceRange { .. } => 3,
            Instruction::NegInt(_, _) => 1,
            Instruction::NotInt(_, _) => 1,
            Instruction::NegLong(_, _) => 1,
            Instruction::NotLong(_, _) => 1,
            Instruction::NegFloat(_, _) => 1,
            Instruction::NegDouble(_, _) => 1,
            Instruction::IntToLong(_, _) => 1,
            Instruction::IntToFloat(_, _) => 1,
            Instruction::IntToDouble(_, _) => 1,
            Instruction::LongToInt(_, _) => 1,
            Instruction::LongToFloat(_, _) => 1,
            Instruction::LongToDouble(_, _) => 1,
            Instruction::FloatToInt(_, _) => 1,
            Instruction::FloatToLong(_, _) => 1,
            Instruction::FloatToDouble(_, _) => 1,
            Instruction::DoubleToInt(_, _) => 1,
            Instruction::DoubleToLong(_, _) => 1,
            Instruction::DoubleToFloat(_, _) => 1,
            Instruction::IntTobyte(_, _) => 1,
            Instruction::IntTochar(_, _) => 1,
            Instruction::IntToshort(_, _) => 1,
            Instruction::AddInt(_, _, _) => 2,
            Instruction::SubInt(_, _, _) => 2,
            Instruction::MulInt(_, _, _) => 2,
            Instruction::DivInt(_, _, _) => 2,
            Instruction::RemInt(_, _, _) => 2,
            Instruction::AndInt(_, _, _) => 2,
            Instruction::OrInt(_, _, _) => 2,
            Instruction::XorInt(_, _, _) => 2,
            Instruction::ShlInt(_, _, _) => 2,
            Instruction::ShrInt(_, _, _) => 2,
            Instruction::UshrInt(_, _, _) => 2,
            Instruction::AddLong(_, _, _) => 2,
            Instruction::SubLong(_, _, _) => 2,
            Instruction::MulLong(_, _, _) => 2,
            Instruction::DivLong(_, _, _) => 2,
            Instruction::RemLong(_, _, _) => 2,
            Instruction::AndLong(_, _, _) => 2,
            Instruction::OrLong(_, _, _) => 2,
            Instruction::XorLong(_, _, _) => 2,
            Instruction::ShlLong(_, _, _) => 2,
            Instruction::ShrLong(_, _, _) => 2,
            Instruction::UshrLong(_, _, _) => 2,
            Instruction::AddFloat(_, _, _) => 2,
            Instruction::SubFloat(_, _, _) => 2,
            Instruction::MulFloat(_, _, _) => 2,
            Instruction::DivFloat(_, _, _) => 2,
            Instruction::RemFloat(_, _, _) => 2,
            Instruction::AddDouble(_, _, _) => 2,
            Instruction::SubDouble(_, _, _) => 2,
            Instruction::MulDouble(_, _, _) => 2,
            Instruction::DivDouble(_, _, _) => 2,
            Instruction::RemDouble(_, _, _) => 2,
            Instruction::AddInt2(_, _) => 1,
            Instruction::SubInt2(_, _) => 1,
            Instruction::MulInt2(_, _) => 1,
            Instruction::DivInt2(_, _) => 1,
            Instruction::RemInt2(_, _) => 1,
            Instruction::AndInt2(_, _) => 1,
            Instruction::OrInt2(_, _) => 1,
            Instruction::XorInt2(_, _) => 1,
            Instruction::ShlInt2(_, _) => 1,
            Instruction::ShrInt2(_, _) => 1,
            Instruction::UShrInt2(_, _) => 1,
            Instruction::AddLong2(_, _) => 1,
            Instruction::SubLong2(_, _) => 1,
            Instruction::MulLong2(_, _) => 1,
            Instruction::DivLong2(_, _) => 1,
            Instruction::RemLong2(_, _) => 1,
            Instruction::AndLong2(_, _) => 1,
            Instruction::OrLong2(_, _) => 1,
            Instruction::XorLong2(_, _) => 1,
            Instruction::ShlLong2(_, _) => 1,
            Instruction::ShrLong2(_, _) => 1,
            Instruction::UShrLong2(_, _) => 1,
            Instruction::AddFloat2(_, _) => 1,
            Instruction::SubFloat2(_, _) => 1,
            Instruction::MulFloat2(_, _) => 1,
            Instruction::DivFloat2(_, _) => 1,
            Instruction::RemFloat2(_, _) => 1,
            Instruction::AddDouble2(_, _) => 1,
            Instruction::SubDouble2(_, _) => 1,
            Instruction::MulDouble2(_, _) => 1,
            Instruction::DivDouble2(_, _) => 1,
            Instruction::RemDouble2(_, _) => 1,
            Instruction::AddInt16(_, _, _) => 2,
            Instruction::RsubInt16(_, _, _) => 2,
            Instruction::MulInt16(_, _, _) => 2,
            Instruction::DivInt16(_, _, _) => 2,
            Instruction::RemInt16(_, _, _) => 2,
            Instruction::AndInt16(_, _, _) => 2,
            Instruction::OrInt16(_, _, _) => 2,
            Instruction::XorInt16(_, _, _) => 2,
            Instruction::AddInt8(_, _, _) => 2,
            Instruction::RsubInt8(_, _, _) => 2,
            Instruction::MulInt8(_, _, _) => 2,
            Instruction::DivInt8(_, _, _) => 2,
            Instruction::RemInt8(_, _, _) => 2,
            Instruction::AndInt8(_, _, _) => 2,
            Instruction::OrInt8(_, _, _) => 2,
            Instruction::XorInt8(_, _, _) => 2,
            Instruction::ShlInt8(_, _, _) => 2,
            Instruction::ShrInt8(_, _, _) => 2,
            Instruction::UshrInt8(_, _, _) => 2,
        }
    }
}

pub(crate) mod opcode {
    macro_rules! mkop {
        ($v:expr => $n:ident) => {
            pub(crate) const $n: u8 = $v;
        };
    }
    mkop!(0x00 => NOP);
    mkop!(0x01 => MOVE);
    mkop!(0x02 => MOVEFROM16);
    mkop!(0x03 => MOVE16);
    mkop!(0x04 => MOVEWIDE);
    mkop!(0x05 => MOVEWIDEFROM16);
    mkop!(0x06 => MOVEWIDE16);
    mkop!(0x07 => MOVEOBJECT);
    mkop!(0x08 => MOVEOBJECTFROM16);
    mkop!(0x09 => MOVEOBJECT16);
    mkop!(0x0a => MOVERESULT);
    mkop!(0x0b => MOVERESULTWIDE);
    mkop!(0x0c => MOVERESULTOBJECT);
    mkop!(0x0d => MOVEEXCEPTION);
    mkop!(0x0e => RETURNVOID);
    mkop!(0x0f => RETURN);
    mkop!(0x10 => RETURNWIDE);
    mkop!(0x11 => RETURNOBJECT);
    mkop!(0x12 => CONST4);
    mkop!(0x13 => CONST16);
    mkop!(0x14 => CONST);
    mkop!(0x15 => CONSTHIGH16);
    mkop!(0x16 => CONSTWIDE16);
    mkop!(0x17 => CONSTWIDE32);
    mkop!(0x18 => CONSTWIDE);
    mkop!(0x19 => CONSTWIDEHIGH16);
    mkop!(0x1a => CONSTSTRING);
    mkop!(0x1b => CONSTSTRINGJUMBO);
    mkop!(0x1c => CONSTCLASS);
    mkop!(0x1d => MONITORENTER);
    mkop!(0x1e => MONITOREXIT);
    mkop!(0x1f => CHECKCAST);
    mkop!(0x20 => INSTANCEOF);
    mkop!(0x21 => ARRAYLENGTH);
    mkop!(0x22 => NEWINSTANCE);
    mkop!(0x23 => NEWARRAY);
    mkop!(0x24 => FILLEDNEWARRAY);
    mkop!(0x25 => FILLEDNEWARRAYRANGE);
    mkop!(0x26 => FILLARRAYDATA);
    mkop!(0x27 => THROW);
    mkop!(0x28 => GOTO);
    mkop!(0x29 => GOTO16);
    mkop!(0x2a => GOTO32);
    mkop!(0x2b => PACKEDSWITCH);
    mkop!(0x2c => SPARSESWITCH);
    mkop!(0x2d => CMPLFLOAT);
    mkop!(0x2e => CMPGFLOAT);
    mkop!(0x2f => CMPLDOUBLE);
    mkop!(0x30 => CMPGDOUBLE);
    mkop!(0x31 => CMPLONG);
    mkop!(0x32 => IFEQ);
    mkop!(0x33 => IFNE);
    mkop!(0x34 => IFLT);
    mkop!(0x35 => IFGE);
    mkop!(0x36 => IFGT);
    mkop!(0x37 => IFLE);
    mkop!(0x38 => IFEQZ);
    mkop!(0x39 => IFNEZ);
    mkop!(0x3a => IFLTZ);
    mkop!(0x3b => IFGEZ);
    mkop!(0x3c => IFGTZ);
    mkop!(0x3d => IFLEZ);
    mkop!(0x44 => AGET);
    mkop!(0x45 => AGETWIDE);
    mkop!(0x46 => AGETOBJECT);
    mkop!(0x47 => AGETBOOLEAN);
    mkop!(0x48 => AGETBYTE);
    mkop!(0x49 => AGETCHAR);
    mkop!(0x4a => AGETSHORT);
    mkop!(0x4b => APUT);
    mkop!(0x4c => APUTWIDE);
    mkop!(0x4d => APUTOBJECT);
    mkop!(0x4e => APUTBOOLEAN);
    mkop!(0x4f => APUTBYTE);
    mkop!(0x50 => APUTCHAR);
    mkop!(0x51 => APUTSHORT);
    mkop!(0x52 => IGET);
    mkop!(0x53 => IGETWIDE);
    mkop!(0x54 => IGETOBJECT);
    mkop!(0x55 => IGETBOOLEAN);
    mkop!(0x56 => IGETBYTE);
    mkop!(0x57 => IGETCHAR);
    mkop!(0x58 => IGETSHORT);
    mkop!(0x59 => IPUT);
    mkop!(0x5a => IPUTWIDE);
    mkop!(0x5b => IPUTOBJECT);
    mkop!(0x5c => IPUTBOOLEAN);
    mkop!(0x5d => IPUTBYTE);
    mkop!(0x5e => IPUTCHAR);
    mkop!(0x5f => IPUTSHORT);
    mkop!(0x60 => SGET);
    mkop!(0x61 => SGETWIDE);
    mkop!(0x62 => SGETOBJECT);
    mkop!(0x63 => SGETBOOLEAN);
    mkop!(0x64 => SGETBYTE);
    mkop!(0x65 => SGETCHAR);
    mkop!(0x66 => SGETSHORT);
    mkop!(0x67 => SPUT);
    mkop!(0x68 => SPUTWIDE);
    mkop!(0x69 => SPUTOBJECT);
    mkop!(0x6a => SPUTBOOLEAN);
    mkop!(0x6b => SPUTBYTE);
    mkop!(0x6c => SPUTCHAR);
    mkop!(0x6d => SPUTSHORT);
    mkop!(0x6e => INVOKEVIRTUAL);
    mkop!(0x6f => INVOKESUPER);
    mkop!(0x70 => INVOKEDIRECT);
    mkop!(0x71 => INVOKESTATIC);
    mkop!(0x72 => INVOKEINTERFACE);
    mkop!(0x74 => INVOKEVIRTUALRANGE);
    mkop!(0x75 => INVOKESUPERRANGE);
    mkop!(0x76 => INVOKEDIRECTRANGE);
    mkop!(0x77 => INVOKESTATICRANGE);
    mkop!(0x78 => INVOKEINTERFACERANGE);
    mkop!(0x7b => NEGINT);
    mkop!(0x7c => NOTINT);
    mkop!(0x7d => NEGLONG);
    mkop!(0x7e => NOTLONG);
    mkop!(0x7f => NEGFLOAT);
    mkop!(0x80 => NEGDOUBLE);
    mkop!(0x81 => INTTOLONG);
    mkop!(0x82 => INTTOFLOAT);
    mkop!(0x83 => INTTODOUBLE);
    mkop!(0x84 => LONGTOINT);
    mkop!(0x85 => LONGTOFLOAT);
    mkop!(0x86 => LONGTODOUBLE);
    mkop!(0x87 => FLOATTOINT);
    mkop!(0x88 => FLOATTOLONG);
    mkop!(0x89 => FLOATTODOUBLE);
    mkop!(0x8a => DOUBLETOINT);
    mkop!(0x8b => DOUBLETOLONG);
    mkop!(0x8c => DOUBLETOFLOAT);
    mkop!(0x8d => INTTOBYTE);
    mkop!(0x8e => INTTOCHAR);
    mkop!(0x8f => INTTOSHORT);
    mkop!(0x90 => ADDINT);
    mkop!(0x91 => SUBINT);
    mkop!(0x92 => MULINT);
    mkop!(0x93 => DIVINT);
    mkop!(0x94 => REMINT);
    mkop!(0x95 => ANDINT);
    mkop!(0x96 => ORINT);
    mkop!(0x97 => XORINT);
    mkop!(0x98 => SHLINT);
    mkop!(0x99 => SHRINT);
    mkop!(0x9a => USHRINT);
    mkop!(0x9b => ADDLONG);
    mkop!(0x9c => SUBLONG);
    mkop!(0x9d => MULLONG);
    mkop!(0x9e => DIVLONG);
    mkop!(0x9f => REMLONG);
    mkop!(0xa0 => ANDLONG);
    mkop!(0xa1 => ORLONG);
    mkop!(0xa2 => XORLONG);
    mkop!(0xa3 => SHLLONG);
    mkop!(0xa4 => SHRLONG);
    mkop!(0xa5 => USHRLONG);
    mkop!(0xa6 => ADDFLOAT);
    mkop!(0xa7 => SUBFLOAT);
    mkop!(0xa8 => MULFLOAT);
    mkop!(0xa9 => DIVFLOAT);
    mkop!(0xaa => REMFLOAT);
    mkop!(0xab => ADDDOUBLE);
    mkop!(0xac => SUBDOUBLE);
    mkop!(0xad => MULDOUBLE);
    mkop!(0xae => DIVDOUBLE);
    mkop!(0xaf => REMDOUBLE);
    mkop!(0xb0 => ADDINT2);
    mkop!(0xb1 => SUBINT2);
    mkop!(0xb2 => MULINT2);
    mkop!(0xb3 => DIVINT2);
    mkop!(0xb4 => REMINT2);
    mkop!(0xb5 => ANDINT2);
    mkop!(0xb6 => ORINT2);
    mkop!(0xb7 => XORINT2);
    mkop!(0xb8 => SHLINT2);
    mkop!(0xb9 => SHRINT2);
    mkop!(0xba => USHRINT2);
    mkop!(0xbb => ADDLONG2);
    mkop!(0xbc => SUBLONG2);
    mkop!(0xbd => MULLONG2);
    mkop!(0xbe => DIVLONG2);
    mkop!(0xbf => REMLONG2);
    mkop!(0xc0 => ANDLONG2);
    mkop!(0xc1 => ORLONG2);
    mkop!(0xc2 => XORLONG2);
    mkop!(0xc3 => SHLLONG2);
    mkop!(0xc4 => SHRLONG2);
    mkop!(0xc5 => USHRLONG2);
    mkop!(0xc6 => ADDFLOAT2);
    mkop!(0xc7 => SUBFLOAT2);
    mkop!(0xc8 => MULFLOAT2);
    mkop!(0xc9 => DIVFLOAT2);
    mkop!(0xca => REMFLOAT2);
    mkop!(0xcb => ADDDOUBLE2);
    mkop!(0xcc => SUBDOUBLE2);
    mkop!(0xcd => MULDOUBLE2);
    mkop!(0xce => DIVDOUBLE2);
    mkop!(0xcf => REMDOUBLE2);
    mkop!(0xd0 => ADDINT16);
    mkop!(0xd1 => RSUBINT16);
    mkop!(0xd2 => MULINT16);
    mkop!(0xd3 => DIVINT16);
    mkop!(0xd4 => REMINT16);
    mkop!(0xd5 => ANDINT16);
    mkop!(0xd6 => ORINT16);
    mkop!(0xd7 => XORINT16);
    mkop!(0xd8 => ADDINT8);
    mkop!(0xd9 => RSUBINT8);
    mkop!(0xda => MULINT8);
    mkop!(0xdb => DIVINT8);
    mkop!(0xdc => REMINT8);
    mkop!(0xdd => ANDINT8);
    mkop!(0xde => ORINT8);
    mkop!(0xdf => XORINT8);
    mkop!(0xe0 => SHLINT8);
    mkop!(0xe1 => SHRINT8);
    mkop!(0xe2 => USHRINT8);
}

/// Decoders for various instruction formats
mod d {
    use super::Error;

    /// Helper to consume a u16 and advance the slice
    pub(crate) fn consume_u16(bytecode: &mut &[u16]) -> Result<u16, Error> {
        let (a, rest) = match *bytecode {
            [a, rest @ ..] => (*a, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        Ok(a)
    }

    /// Helper to consume a u32 and advance the slice
    pub(crate) fn consume_u32(bytecode: &mut &[u16]) -> Result<u32, Error> {
        let (al, ah, rest) = match *bytecode {
            [al, ah, rest @ ..] => (*al, *ah, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let a = (ah as u32) << 16 | al as u32;

        Ok(a)
    }

    /// AA|op
    ///
    /// returns AA
    ///
    /// decodes formats 11x, 10t
    pub(crate) fn aa_op(bytecode: &mut &[u16]) -> Result<u8, Error> {
        let (a, rest) = match *bytecode {
            [a, rest @ ..] => (*a, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let a = (a >> 8) as u8;

        Ok(a)
    }

    /// B|A|op
    ///
    /// returns (B, A)
    ///
    /// decodes formats 11x, 10t
    pub(crate) fn ba_op(bytecode: &mut &[u16]) -> Result<(u8, u8), Error> {
        let ab = aa_op(bytecode)?;
        let b = ab >> 4;
        let a = ab & 0xf;

        Ok((b, a))
    }

    /// ØØ|op
    ///
    /// returns ()
    ///
    /// decodes formats 10x
    pub(crate) fn zz_op(bytecode: &mut &[u16]) -> Result<(), Error> {
        let aa = aa_op(bytecode)?;

        if aa != 0 {
            return Err(Error::Encoding);
        }

        Ok(())
    }

    /// AA|op BBBB
    ///
    /// returns (AA, BBBB)
    ///
    /// decodes formats 20bc, 22x, 21t, 21s, 21h, 21c
    pub(crate) fn aa_op_bbbb(bytecode: &mut &[u16]) -> Result<(u8, u16), Error> {
        let (a, bbbb, rest) = match *bytecode {
            [a, bbbb, rest @ ..] => (*a, *bbbb, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let a = (a >> 8) as u8;

        Ok((a, bbbb))
    }

    /// AA|op BB|BB
    ///
    /// returns (AA, CC, BB)
    ///
    /// decodes formats 23x, 22b
    pub(crate) fn aa_op_ccbb(bytecode: &mut &[u16]) -> Result<(u8, u8, u8), Error> {
        let (aa, ccbb) = aa_op_bbbb(bytecode)?;

        let cc = (ccbb >> 8) as u8;
        let bb = ccbb as u8;

        Ok((aa, cc, bb))
    }

    /// B|A|op CCCC
    ///
    /// returns (B, A, CCCC)
    ///
    /// decodes formats 22t, 22s, 22c, 22cs
    pub(crate) fn ba_op_cccc(bytecode: &mut &[u16]) -> Result<(u8, u8, u16), Error> {
        let (ba, cccc) = aa_op_bbbb(bytecode)?;
        let b = ba >> 4;
        let a = ba & 0xf;

        Ok((b, a, cccc))
    }

    /// ØØ|op AAAA
    ///
    /// returns (AAAA)
    ///
    /// decodes formats 20t
    pub(crate) fn zz_op_aaaa(bytecode: &mut &[u16]) -> Result<u16, Error> {
        let (zz, aaaa) = aa_op_bbbb(bytecode)?;

        if zz != 0 {
            return Err(Error::Encoding);
        }

        Ok(aaaa)
    }

    /// AA|op BBBBBBBB
    ///
    /// returns (AA, BBBBBBBB)
    ///
    /// decodes formats 31i, 31t, 31c
    pub(crate) fn aa_op_bbbbbbbb(bytecode: &mut &[u16]) -> Result<(u8, u32), Error> {
        let (a, bl, bh, rest) = match *bytecode {
            [a, bl, bh, rest @ ..] => (*a, *bl, *bh, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let a = (a >> 8) as u8;
        let b = (bh as u32) << 16 | bl as u32;

        Ok((a, b))
    }

    /// ØØ|op AAAAAAAA
    ///
    /// returns (AAAAAAAA)
    ///
    /// decodes formats 30t
    pub(crate) fn zz_op_aaaaaaaa(bytecode: &mut &[u16]) -> Result<u32, Error> {
        let (zz, aaaaaaaa) = aa_op_bbbbbbbb(bytecode)?;

        if zz != 0 {
            return Err(Error::Encoding);
        }

        Ok(aaaaaaaa)
    }

    /// AA|op CCCC|BBBB
    ///
    /// returns (AA, CCCC, BBBB)
    ///
    /// decodes formats 3rc, 3rms, 3rmi
    ///
    /// ERRATA: This instruction format is documented incorrectly in the "Dalvik
    /// executable instruction formats" manual as "AA|op BBBB|CCCC"
    pub(crate) fn aa_op_ccccbbbb(bytecode: &mut &[u16]) -> Result<(u8, u16, u16), Error> {
        let (aa, ccccbbbb) = aa_op_bbbbbbbb(bytecode)?;

        let cccc = (ccccbbbb >> 16) as u16;
        let bbbb = ccccbbbb as u16;

        Ok((aa, bbbb, cccc))
    }

    /// A|G|op BBBB F|E|D|C
    ///
    /// returns (A, G, BBBB, F, E, D, C)
    ///
    /// decodes formats 35c, 35ms, 35mi
    pub(crate) fn ag_op_bbbbfedc(bytecode: &mut &[u16]) -> Result<(u8, u8, u16, u8, u8, u8, u8), Error> {
        let (agop, bbbb, fedc, rest) = match *bytecode {
            [agop, b, fedc, rest @ ..] => (*agop, *b, *fedc, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let a = ((agop >> 12) & 0xf) as u8;
        let g = ((agop >> 8) & 0xf) as u8;
        let f = ((fedc >> 12) & 0xf) as u8;
        let e = ((fedc >> 8) & 0xf) as u8;
        let d = ((fedc >> 4) & 0xf) as u8;
        let c = ((fedc >> 0) & 0xf) as u8;

        Ok((a, g, bbbb, f, e, d, c))
    }

    /// ØØ|op AAAA BBBB
    ///
    /// returns (AAAA, BBBB)
    ///
    /// decodes formats 32x
    pub(crate) fn zz_op_aaaabbbb(bytecode: &mut &[u16]) -> Result<(u16, u16), Error> {
        let (zz, aaaa, bbbb) = aa_op_ccccbbbb(bytecode)?;

        if zz != 0 {
            return Err(Error::Encoding);
        }

        Ok((aaaa, bbbb))
    }

    /// AA|op BBBBBBBBBBBBBBBB
    ///
    /// returns (AA, BBBBBBBBBBBBBBBB)
    ///
    /// decodes formats 51l
    pub(crate) fn aa_op_bbbbbbbbbbbbbbbb(bytecode: &mut &[u16]) -> Result<(u8, u64), Error> {
        let (aa, b0, b1, b2, b3, rest) = match *bytecode {
            [a, b0, b1, b2, b3, rest @ ..] => (*a, *b0, *b1, *b2, *b3, rest),
            _ => return Err(Error::Truncated),
        };
        *bytecode = rest;

        let aa = (aa >> 8) as u8;
        #[rustfmt::skip]
        let bbbbbbbbbbbbbbbb = (b3 as u64) << (3 * 16)
                             | (b2 as u64) << (2 * 16)
                             | (b1 as u64) << (1 * 16)
                             | (b0 as u64) << (0 * 16);

        Ok((aa, bbbbbbbbbbbbbbbb))
    }
}
