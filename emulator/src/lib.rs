//! PYNQ学習用CPU ISAの参照エミュレータ。

use std::fmt;

pub const ROM_SIZE: usize = 0x4000;
pub const RAM_BASE: u32 = 0x4000;
pub const RAM_END: u32 = 0x1_0000;
pub const UART_TX: u32 = 0x8000_0000;
pub const UART_STATUS: u32 = 0x8000_0004;
pub const SIM_EXIT: u32 = 0x8000_1000;

pub mod opcode {
    pub const NOP: u8 = 0x00; pub const ADD: u8 = 0x01; pub const SUB: u8 = 0x02;
    pub const AND: u8 = 0x03; pub const OR: u8 = 0x04; pub const XOR: u8 = 0x05;
    pub const SHL: u8 = 0x06; pub const SHR: u8 = 0x07; pub const SAR: u8 = 0x08;
    pub const MOVI: u8 = 0x09; pub const ADDI: u8 = 0x0a; pub const LUI: u8 = 0x0b;
    pub const LOAD: u8 = 0x0c; pub const STORE: u8 = 0x0d; pub const LOADB: u8 = 0x0e;
    pub const STOREB: u8 = 0x0f; pub const BEQ: u8 = 0x10; pub const BNE: u8 = 0x11;
    pub const BLT: u8 = 0x12; pub const BGE: u8 = 0x13; pub const JMP: u8 = 0x14;
    pub const CALL: u8 = 0x15; pub const RET: u8 = 0x16; pub const HALT: u8 = 0x17;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Nop,
    R { op: u8, rd: usize, rs1: usize, rs2: usize },
    Movi { rd: usize, imm: i32 },
    Addi { rd: usize, rs1: usize, imm: i32 },
    Lui { rd: usize, imm: u16 },
    Memory { op: u8, reg: usize, base: usize, offset: i32 },
    Branch { op: u8, rs1: usize, rs2: usize, offset: i32 },
    Jump { call: bool, offset: i32 },
    Ret,
    Halt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fault {
    ProgramTooLarge(usize),
    FetchViolation(u32),
    IllegalInstruction { pc: u32, word: u32 },
    UnalignedAccess(u32),
    MemoryViolation(u32),
    StepLimit(u64),
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramTooLarge(n) => write!(f, "プログラムがROM容量を超えています（{n} byte）"),
            Self::FetchViolation(a) => write!(f, "命令フェッチ違反: 0x{a:08x}"),
            Self::IllegalInstruction { pc, word } => write!(f, "不正命令: PC=0x{pc:08x}, word=0x{word:08x}"),
            Self::UnalignedAccess(a) => write!(f, "未アラインアクセス: 0x{a:08x}"),
            Self::MemoryViolation(a) => write!(f, "メモリアクセス違反: 0x{a:08x}"),
            Self::StepLimit(n) => write!(f, "実行ステップ上限（{n}）に到達しました"),
        }
    }
}

impl std::error::Error for Fault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason { Halt, SimExit(u32) }

fn sign_extend(value: u32, bits: u32) -> i32 {
    ((value << (32 - bits)) as i32) >> (32 - bits)
}

pub fn decode(word: u32, pc: u32) -> Result<Instruction, Fault> {
    use opcode::*;
    let op = (word >> 26) as u8;
    let a = ((word >> 22) & 15) as usize;
    let b = ((word >> 18) & 15) as usize;
    let c = ((word >> 14) & 15) as usize;
    let illegal = || Fault::IllegalInstruction { pc, word };
    Ok(match op {
        NOP if word & 0x03ff_ffff == 0 => Instruction::Nop,
        ADD..=SAR if word & 0x3fff == 0 => Instruction::R { op, rd: a, rs1: b, rs2: c },
        MOVI => Instruction::Movi { rd: a, imm: sign_extend(word & 0x3f_ffff, 22) },
        ADDI => Instruction::Addi { rd: a, rs1: b, imm: sign_extend(word & 0x3_ffff, 18) },
        LUI if word & 0x003f_0000 == 0 => Instruction::Lui { rd: a, imm: word as u16 },
        LOAD | STORE | LOADB | STOREB => Instruction::Memory {
            op, reg: a, base: b, offset: sign_extend(word & 0x3_ffff, 18),
        },
        BEQ | BNE | BLT | BGE => Instruction::Branch {
            op, rs1: a, rs2: b, offset: sign_extend(word & 0x3_ffff, 18),
        },
        JMP | CALL => Instruction::Jump { call: op == CALL, offset: sign_extend(word & 0x03ff_ffff, 26) },
        RET if word & 0x03ff_ffff == 0 => Instruction::Ret,
        HALT if word & 0x03ff_ffff == 0 => Instruction::Halt,
        _ => return Err(illegal()),
    })
}

pub struct Cpu {
    pub registers: [u32; 16],
    pub pc: u32,
    pub steps: u64,
    pub uart_output: Vec<u8>,
    rom: Vec<u8>,
    ram: Vec<u8>,
    trace: bool,
}

impl Cpu {
    pub fn new(program: &[u8], trace: bool) -> Result<Self, Fault> {
        if program.len() > ROM_SIZE { return Err(Fault::ProgramTooLarge(program.len())); }
        let mut rom = vec![0; ROM_SIZE];
        rom[..program.len()].copy_from_slice(program);
        let mut registers = [0; 16];
        registers[15] = RAM_END;
        Ok(Self { registers, pc: 0, steps: 0, uart_output: Vec::new(), rom,
            ram: vec![0; (RAM_END - RAM_BASE) as usize], trace })
    }

    fn fetch(&self) -> Result<u32, Fault> {
        if self.pc & 3 != 0 || self.pc as usize + 4 > ROM_SIZE {
            return Err(Fault::FetchViolation(self.pc));
        }
        Ok(u32::from_le_bytes(self.rom[self.pc as usize..self.pc as usize + 4].try_into().unwrap()))
    }

    fn set_reg(&mut self, index: usize, value: u32) {
        if index != 0 { self.registers[index] = value; }
    }

    fn ram_index(&self, address: u32, bytes: u32) -> Result<usize, Fault> {
        let end = address.checked_add(bytes).ok_or(Fault::MemoryViolation(address))?;
        if address < RAM_BASE || end > RAM_END { return Err(Fault::MemoryViolation(address)); }
        Ok((address - RAM_BASE) as usize)
    }

    fn load_word(&self, address: u32) -> Result<u32, Fault> {
        if address & 3 != 0 { return Err(Fault::UnalignedAccess(address)); }
        if address == UART_STATUS { return Ok(1); }
        let i = self.ram_index(address, 4)?;
        Ok(u32::from_le_bytes(self.ram[i..i + 4].try_into().unwrap()))
    }

    fn load_byte(&self, address: u32) -> Result<u32, Fault> {
        if address == UART_STATUS { return Ok(1); }
        Ok(self.ram[self.ram_index(address, 1)?] as u32)
    }

    fn store(&mut self, address: u32, value: u32, byte: bool) -> Result<Option<StopReason>, Fault> {
        if !byte && address & 3 != 0 { return Err(Fault::UnalignedAccess(address)); }
        if address == UART_TX {
            self.uart_output.push(value as u8);
            return Ok(None);
        }
        if address == SIM_EXIT {
            return Ok(Some(StopReason::SimExit(value)));
        }
        let width = if byte { 1 } else { 4 };
        let i = self.ram_index(address, width)?;
        if byte { self.ram[i] = value as u8; }
        else { self.ram[i..i + 4].copy_from_slice(&value.to_le_bytes()); }
        Ok(None)
    }

    pub fn step(&mut self) -> Result<Option<StopReason>, Fault> {
        use opcode::*;
        let instruction_pc = self.pc;
        let word = self.fetch()?;
        let instruction = decode(word, instruction_pc)?;
        if self.trace { eprintln!("PC={instruction_pc:08x} IR={word:08x} {instruction:?}"); }
        self.pc = self.pc.wrapping_add(4);
        self.steps += 1;
        match instruction {
            Instruction::Nop => {}
            Instruction::R { op, rd, rs1, rs2 } => {
                let x = self.registers[rs1]; let y = self.registers[rs2];
                let value = match op {
                    ADD => x.wrapping_add(y), SUB => x.wrapping_sub(y), AND => x & y,
                    OR => x | y, XOR => x ^ y, SHL => x.wrapping_shl(y & 31),
                    SHR => x.wrapping_shr(y & 31), SAR => ((x as i32) >> (y & 31)) as u32,
                    _ => unreachable!(),
                };
                self.set_reg(rd, value);
            }
            Instruction::Movi { rd, imm } => self.set_reg(rd, imm as u32),
            Instruction::Addi { rd, rs1, imm } => self.set_reg(rd, self.registers[rs1].wrapping_add(imm as u32)),
            Instruction::Lui { rd, imm } => self.set_reg(rd, (imm as u32) << 16),
            Instruction::Memory { op, reg, base, offset } => {
                let address = self.registers[base].wrapping_add(offset as u32);
                match op {
                    LOAD => { let value = self.load_word(address)?; self.set_reg(reg, value); }
                    LOADB => { let value = self.load_byte(address)?; self.set_reg(reg, value); }
                    STORE | STOREB => if let Some(reason) = self.store(address, self.registers[reg], op == STOREB)? { return Ok(Some(reason)); },
                    _ => unreachable!(),
                }
            }
            Instruction::Branch { op, rs1, rs2, offset } => {
                let x = self.registers[rs1]; let y = self.registers[rs2];
                let taken = match op { BEQ => x == y, BNE => x != y, BLT => (x as i32) < (y as i32), BGE => (x as i32) >= (y as i32), _ => unreachable!() };
                if taken { self.pc = self.pc.wrapping_add((offset as u32) << 2); }
            }
            Instruction::Jump { call, offset } => {
                if call { self.set_reg(14, self.pc); }
                self.pc = self.pc.wrapping_add((offset as u32) << 2);
            }
            Instruction::Ret => self.pc = self.registers[14],
            Instruction::Halt => return Ok(Some(StopReason::Halt)),
        }
        self.registers[0] = 0;
        Ok(None)
    }

    pub fn run(&mut self, max_steps: u64) -> Result<StopReason, Fault> {
        while self.steps < max_steps {
            if let Some(reason) = self.step()? { return Ok(reason); }
        }
        Err(Fault::StepLimit(max_steps))
    }

    pub fn register_dump(&self) -> String {
        (0..16).map(|i| format!("r{i:02}=0x{:08x}{}", self.registers[i], if i % 4 == 3 { "\n" } else { "  " })).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(words: &[u32]) -> Vec<u8> { words.iter().flat_map(|x| x.to_le_bytes()).collect() }
    fn r(op: u8, rd: u32, a: u32, b: u32) -> u32 { (op as u32) << 26 | rd << 22 | a << 18 | b << 14 }
    fn movi(rd: u32, value: i32) -> u32 { (opcode::MOVI as u32) << 26 | rd << 22 | (value as u32 & 0x3f_ffff) }

    #[test]
    fn 算術論理とr0固定を実行する() {
        use opcode::*;
        let code = image(&[movi(1, 40), movi(2, 2), r(ADD, 3, 1, 2), r(SUB, 4, 1, 2),
            r(AND, 5, 1, 2), r(OR, 6, 1, 2), r(XOR, 7, 1, 2), r(ADD, 0, 1, 2), (HALT as u32) << 26]);
        let mut cpu = Cpu::new(&code, false).unwrap();
        assert_eq!(cpu.run(20), Ok(StopReason::Halt));
        assert_eq!((cpu.registers[3], cpu.registers[4], cpu.registers[5]), (42, 38, 0));
        assert_eq!((cpu.registers[6], cpu.registers[7], cpu.registers[0]), (42, 42, 0));
    }

    #[test]
    fn シフトの符号を区別する() {
        use opcode::*;
        let code = image(&[movi(1, -8), movi(2, 2), r(SHL, 3, 1, 2), r(SHR, 4, 1, 2), r(SAR, 5, 1, 2), (HALT as u32) << 26]);
        let mut cpu = Cpu::new(&code, false).unwrap(); cpu.run(10).unwrap();
        assert_eq!(cpu.registers[3], 0xffff_ffe0); assert_eq!(cpu.registers[4], 0x3fff_fffe); assert_eq!(cpu.registers[5], 0xffff_fffe);
    }

    #[test]
    fn 不正命令と未アラインを検出する() {
        assert!(matches!(Cpu::new(&image(&[1]), false).unwrap().step(), Err(Fault::IllegalInstruction { .. })));
        let code = image(&[movi(1, RAM_BASE as i32 + 1), (opcode::LOAD as u32) << 26 | 2 << 22 | 1 << 18]);
        let mut cpu = Cpu::new(&code, false).unwrap(); cpu.step().unwrap();
        assert_eq!(cpu.step(), Err(Fault::UnalignedAccess(RAM_BASE + 1)));
    }
}
