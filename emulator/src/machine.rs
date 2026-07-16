//! Indigo32 ISAとIndigoOS向け機能の参照実装。

use std::{collections::VecDeque, fmt};

pub const ROM_SIZE: usize = 0x4000;
pub const RAM_BASE: u32 = 0x4000;
pub const RAM_END: u32 = 0x1_0000;
pub const UART_TX: u32 = 0x8000_0000;
pub const UART_STATUS: u32 = 0x8000_0004;
pub const UART_RX_DATA: u32 = 0x8000_0008;
pub const UART_RX_STATUS: u32 = 0x8000_000c;
pub const UART_RX_CONTROL: u32 = 0x8000_0010;
pub const SIM_EXIT: u32 = 0x8000_1000;
pub const UART_FIFO_DEPTH: usize = 16;

pub mod opcode {
    pub const NOP: u8 = 0x00;
    pub const ADD: u8 = 0x01;
    pub const SUB: u8 = 0x02;
    pub const AND: u8 = 0x03;
    pub const OR: u8 = 0x04;
    pub const XOR: u8 = 0x05;
    pub const SHL: u8 = 0x06;
    pub const SHR: u8 = 0x07;
    pub const SAR: u8 = 0x08;
    pub const MOVI: u8 = 0x09;
    pub const ADDI: u8 = 0x0a;
    pub const LUI: u8 = 0x0b;
    pub const LOAD: u8 = 0x0c;
    pub const STORE: u8 = 0x0d;
    pub const LOADB: u8 = 0x0e;
    pub const STOREB: u8 = 0x0f;
    pub const BEQ: u8 = 0x10;
    pub const BNE: u8 = 0x11;
    pub const BLT: u8 = 0x12;
    pub const BGE: u8 = 0x13;
    pub const JMP: u8 = 0x14;
    pub const CALL: u8 = 0x15;
    pub const RET: u8 = 0x16;
    pub const HALT: u8 = 0x17;
    pub const CSRR: u8 = 0x18;
    pub const CSRW: u8 = 0x19;
    pub const ERET: u8 = 0x1a;
    pub const ECALL: u8 = 0x1b;
    pub const WFI: u8 = 0x1c;
}

pub mod csr {
    pub const STATUS: u8 = 0x00;
    pub const EPC: u8 = 0x01;
    pub const CAUSE: u8 = 0x02;
    pub const TVEC: u8 = 0x03;
    pub const BADADDR: u8 = 0x04;
    pub const TIMER_COUNT_LO: u8 = 0x05;
    pub const TIMER_COUNT_HI: u8 = 0x06;
    pub const TIMER_COMPARE_LO: u8 = 0x07;
    pub const TIMER_COMPARE_HI: u8 = 0x08;
    pub const INTERRUPT_PENDING: u8 = 0x09;
    pub const INTERRUPT_ENABLE: u8 = 0x0a;
    pub const USER_BASE: u8 = 0x0b;
    pub const USER_LIMIT: u8 = 0x0c;
    pub const KERNEL_SP: u8 = 0x0d;
    pub const SCRATCH: u8 = 0x0e;
    pub const TIMER_CONTROL: u8 = 0x0f;
    pub const COUNT: usize = 16;
}

pub mod status {
    pub const IE: u32 = 1 << 0;
    pub const PIE: u32 = 1 << 1;
    pub const PRIVILEGED: u32 = 1 << 2;
    pub const PREVIOUS_PRIVILEGED: u32 = 1 << 3;
    pub const MASK: u32 = IE | PIE | PRIVILEGED | PREVIOUS_PRIVILEGED;
}

pub mod interrupt {
    pub const TIMER: u32 = 1 << 0;
    pub const UART_RX: u32 = 1 << 1;
    pub const SOFTWARE: u32 = 1 << 2;
    pub const MASK: u32 = TIMER | UART_RX | SOFTWARE;
}

pub mod cause {
    pub const ILLEGAL_INSTRUCTION: u32 = 0;
    pub const INSTRUCTION_MISALIGNED: u32 = 1;
    pub const INSTRUCTION_ACCESS: u32 = 2;
    pub const LOAD_MISALIGNED: u32 = 3;
    pub const LOAD_ACCESS: u32 = 4;
    pub const STORE_MISALIGNED: u32 = 5;
    pub const STORE_ACCESS: u32 = 6;
    pub const ECALL: u32 = 7;
    pub const TIMER_INTERRUPT: u32 = 8;
    pub const UART_RX_INTERRUPT: u32 = 9;
    pub const SOFTWARE_INTERRUPT: u32 = 10;
    pub const PRIVILEGED_INSTRUCTION: u32 = 11;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Nop,
    R {
        op: u8,
        rd: usize,
        rs1: usize,
        rs2: usize,
    },
    Movi {
        rd: usize,
        imm: i32,
    },
    Addi {
        rd: usize,
        rs1: usize,
        imm: i32,
    },
    Lui {
        rd: usize,
        imm: u16,
    },
    Memory {
        op: u8,
        reg: usize,
        base: usize,
        offset: i32,
    },
    Branch {
        op: u8,
        rs1: usize,
        rs2: usize,
        offset: i32,
    },
    Jump {
        call: bool,
        offset: i32,
    },
    Ret,
    Halt,
    CsrRead {
        rd: usize,
        csr: u8,
    },
    CsrWrite {
        rs: usize,
        csr: u8,
    },
    Eret,
    Ecall,
    Wfi,
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
            Self::IllegalInstruction { pc, word } => {
                write!(f, "不正命令: PC=0x{pc:08x}, word=0x{word:08x}")
            }
            Self::UnalignedAccess(a) => write!(f, "未アラインアクセス: 0x{a:08x}"),
            Self::MemoryViolation(a) => write!(f, "メモリアクセス違反: 0x{a:08x}"),
            Self::StepLimit(n) => write!(f, "実行ステップ上限（{n}）に到達しました"),
        }
    }
}

impl std::error::Error for Fault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Halt,
    SimExit(u32),
}

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
        ADD..=SAR if word & 0x3fff == 0 => Instruction::R {
            op,
            rd: a,
            rs1: b,
            rs2: c,
        },
        MOVI => Instruction::Movi {
            rd: a,
            imm: sign_extend(word & 0x3f_ffff, 22),
        },
        ADDI => Instruction::Addi {
            rd: a,
            rs1: b,
            imm: sign_extend(word & 0x3_ffff, 18),
        },
        LUI if word & 0x003f_0000 == 0 => Instruction::Lui {
            rd: a,
            imm: word as u16,
        },
        LOAD | STORE | LOADB | STOREB => Instruction::Memory {
            op,
            reg: a,
            base: b,
            offset: sign_extend(word & 0x3_ffff, 18),
        },
        BEQ | BNE | BLT | BGE => Instruction::Branch {
            op,
            rs1: a,
            rs2: b,
            offset: sign_extend(word & 0x3_ffff, 18),
        },
        JMP | CALL => Instruction::Jump {
            call: op == CALL,
            offset: sign_extend(word & 0x03ff_ffff, 26),
        },
        RET if word & 0x03ff_ffff == 0 => Instruction::Ret,
        HALT if word & 0x03ff_ffff == 0 => Instruction::Halt,
        CSRR if word & 0x003f_ff00 == 0 => Instruction::CsrRead {
            rd: a,
            csr: word as u8,
        },
        CSRW if word & 0x003f_ff00 == 0 => Instruction::CsrWrite {
            rs: a,
            csr: word as u8,
        },
        ERET if word & 0x03ff_ffff == 0 => Instruction::Eret,
        ECALL if word & 0x03ff_ffff == 0 => Instruction::Ecall,
        WFI if word & 0x03ff_ffff == 0 => Instruction::Wfi,
        _ => return Err(illegal()),
    })
}

#[derive(Debug, Clone)]
pub struct CsrFile {
    values: [u32; csr::COUNT],
    timer_count: u64,
    timer_compare: u64,
}

impl CsrFile {
    fn new() -> Self {
        let mut values = [0; csr::COUNT];
        values[csr::STATUS as usize] = status::PRIVILEGED;
        values[csr::KERNEL_SP as usize] = RAM_END;
        Self {
            values,
            timer_count: 0,
            timer_compare: u64::MAX,
        }
    }

    pub fn read(&self, number: u8) -> Option<u32> {
        match number {
            csr::TIMER_COUNT_LO => Some(self.timer_count as u32),
            csr::TIMER_COUNT_HI => Some((self.timer_count >> 32) as u32),
            csr::TIMER_COMPARE_LO => Some(self.timer_compare as u32),
            csr::TIMER_COMPARE_HI => Some((self.timer_compare >> 32) as u32),
            n if (n as usize) < csr::COUNT => Some(self.values[n as usize]),
            _ => None,
        }
    }

    fn write(&mut self, number: u8, value: u32) -> bool {
        match number {
            csr::STATUS => self.values[number as usize] = value & status::MASK,
            csr::EPC => self.values[number as usize] = value,
            csr::TVEC => {
                if value & 3 != 0 {
                    return false;
                }
                self.values[number as usize] = value;
            }
            csr::TIMER_COMPARE_LO => {
                self.timer_compare = (self.timer_compare & 0xffff_ffff_0000_0000) | value as u64;
            }
            csr::TIMER_COMPARE_HI => {
                self.timer_compare =
                    (self.timer_compare & 0x0000_0000_ffff_ffff) | ((value as u64) << 32);
            }
            csr::INTERRUPT_PENDING => {
                let pending = &mut self.values[number as usize];
                *pending &= !(value & (interrupt::TIMER | interrupt::UART_RX));
                *pending = (*pending & !interrupt::SOFTWARE) | (value & interrupt::SOFTWARE);
            }
            csr::INTERRUPT_ENABLE => self.values[number as usize] = value & interrupt::MASK,
            csr::USER_BASE | csr::USER_LIMIT | csr::KERNEL_SP | csr::SCRATCH => {
                self.values[number as usize] = value;
            }
            csr::TIMER_CONTROL => self.values[number as usize] = value & 1,
            csr::CAUSE | csr::BADADDR | csr::TIMER_COUNT_LO | csr::TIMER_COUNT_HI => return false,
            _ => return false,
        }
        true
    }
}

pub struct Cpu {
    pub registers: [u32; 16],
    pub pc: u32,
    pub steps: u64,
    pub cycles: u64,
    pub uart_output: Vec<u8>,
    pub csrs: CsrFile,
    rom: Vec<u8>,
    ram: Vec<u8>,
    uart_input: VecDeque<u8>,
    uart_source: VecDeque<u8>,
    uart_rx_control: u32,
    uart_rx_overrun: bool,
    waiting_for_interrupt: bool,
    trace: bool,
    trace_interrupts: bool,
    trace_syscalls: bool,
}

impl Cpu {
    pub fn new(program: &[u8], trace: bool) -> Result<Self, Fault> {
        if program.len() > ROM_SIZE {
            return Err(Fault::ProgramTooLarge(program.len()));
        }
        let mut rom = vec![0; ROM_SIZE];
        rom[..program.len()].copy_from_slice(program);
        let mut registers = [0; 16];
        registers[15] = RAM_END;
        Ok(Self {
            registers,
            pc: 0,
            steps: 0,
            cycles: 0,
            uart_output: Vec::new(),
            csrs: CsrFile::new(),
            rom,
            ram: vec![0; (RAM_END - RAM_BASE) as usize],
            uart_input: VecDeque::with_capacity(UART_FIFO_DEPTH),
            uart_source: VecDeque::new(),
            uart_rx_control: 0,
            uart_rx_overrun: false,
            waiting_for_interrupt: false,
            trace,
            trace_interrupts: false,
            trace_syscalls: false,
        })
    }

    pub fn set_trace_interrupts(&mut self, enabled: bool) {
        self.trace_interrupts = enabled;
    }

    pub fn set_trace_syscalls(&mut self, enabled: bool) {
        self.trace_syscalls = enabled;
    }

    pub fn inject_uart_input(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.uart_input.len() == UART_FIFO_DEPTH {
                self.uart_rx_overrun = true;
            } else {
                self.uart_input.push_back(byte);
            }
        }
        self.update_pending();
    }

    pub fn queue_uart_input(&mut self, bytes: &[u8]) {
        self.uart_source.extend(bytes.iter().copied());
    }

    pub fn csr_value(&self, number: u8) -> Option<u32> {
        self.csrs.read(number)
    }

    pub fn set_csr_for_test(&mut self, number: u8, value: u32) -> bool {
        self.csrs.write(number, value)
    }

    fn kernel_mode(&self) -> bool {
        self.csrs.values[csr::STATUS as usize] & status::PRIVILEGED != 0
    }

    fn user_range_allows(&self, address: u32, bytes: u32) -> bool {
        if self.kernel_mode() {
            return true;
        }
        let Some(end) = address.checked_add(bytes) else {
            return false;
        };
        address < 0x8000_0000
            && address >= self.csrs.values[csr::USER_BASE as usize]
            && end <= self.csrs.values[csr::USER_LIMIT as usize]
    }

    fn fetch(&self) -> Result<u32, (u32, Fault)> {
        if self.pc & 3 != 0 {
            return Err((
                cause::INSTRUCTION_MISALIGNED,
                Fault::FetchViolation(self.pc),
            ));
        }
        if self.pc as usize + 4 > ROM_SIZE || !self.user_range_allows(self.pc, 4) {
            return Err((cause::INSTRUCTION_ACCESS, Fault::FetchViolation(self.pc)));
        }
        Ok(u32::from_le_bytes(
            self.rom[self.pc as usize..self.pc as usize + 4]
                .try_into()
                .expect("フェッチ幅は事前検査済み"),
        ))
    }

    fn set_reg(&mut self, index: usize, value: u32) {
        if index != 0 {
            self.registers[index] = value;
        }
    }

    fn ram_index(&self, address: u32, bytes: u32) -> Result<usize, Fault> {
        let end = address
            .checked_add(bytes)
            .ok_or(Fault::MemoryViolation(address))?;
        if address < RAM_BASE || end > RAM_END || !self.user_range_allows(address, bytes) {
            return Err(Fault::MemoryViolation(address));
        }
        Ok((address - RAM_BASE) as usize)
    }

    fn load_word(&mut self, address: u32) -> Result<u32, Fault> {
        if address & 3 != 0 {
            return Err(Fault::UnalignedAccess(address));
        }
        if !self.kernel_mode() && address >= 0x8000_0000 {
            return Err(Fault::MemoryViolation(address));
        }
        match address {
            UART_STATUS => return Ok(1),
            UART_RX_DATA => return Ok(self.uart_input.pop_front().unwrap_or(0) as u32),
            UART_RX_STATUS => {
                return Ok(
                    u32::from(!self.uart_input.is_empty()) | (u32::from(self.uart_rx_overrun) << 1)
                );
            }
            UART_RX_CONTROL => return Ok(self.uart_rx_control),
            _ => {}
        }
        let i = self.ram_index(address, 4)?;
        Ok(u32::from_le_bytes(
            self.ram[i..i + 4]
                .try_into()
                .expect("RAM範囲は事前検査済み"),
        ))
    }

    fn load_byte(&mut self, address: u32) -> Result<u32, Fault> {
        if !self.kernel_mode() && address >= 0x8000_0000 {
            return Err(Fault::MemoryViolation(address));
        }
        match address {
            UART_STATUS => return Ok(1),
            UART_RX_DATA => return Ok(self.uart_input.pop_front().unwrap_or(0) as u32),
            UART_RX_STATUS => {
                return Ok(
                    u32::from(!self.uart_input.is_empty()) | (u32::from(self.uart_rx_overrun) << 1)
                );
            }
            UART_RX_CONTROL => return Ok(self.uart_rx_control & 0xff),
            _ => {}
        }
        Ok(self.ram[self.ram_index(address, 1)?] as u32)
    }

    fn store(&mut self, address: u32, value: u32, byte: bool) -> Result<Option<StopReason>, Fault> {
        if !byte && address & 3 != 0 {
            return Err(Fault::UnalignedAccess(address));
        }
        if !self.kernel_mode() && address >= 0x8000_0000 {
            return Err(Fault::MemoryViolation(address));
        }
        if address == UART_TX {
            self.uart_output.push(value as u8);
            return Ok(None);
        }
        if address == UART_RX_CONTROL {
            self.uart_rx_control = value & 1;
            if value & 2 != 0 {
                self.uart_rx_overrun = false;
            }
            return Ok(None);
        }
        if address == SIM_EXIT {
            return Ok(Some(StopReason::SimExit(value)));
        }
        let width = if byte { 1 } else { 4 };
        let i = self.ram_index(address, width)?;
        if byte {
            self.ram[i] = value as u8;
        } else {
            self.ram[i..i + 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(None)
    }

    fn tick(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);
        self.csrs.timer_count = self.csrs.timer_count.wrapping_add(1);
        if self.cycles % 64 == 0 && self.uart_input.len() < UART_FIFO_DEPTH {
            if let Some(byte) = self.uart_source.pop_front() {
                self.uart_input.push_back(byte);
            }
        }
        self.update_pending();
    }

    fn update_pending(&mut self) {
        let timer_active = self.csrs.values[csr::TIMER_CONTROL as usize] & 1 != 0
            && self.csrs.timer_count >= self.csrs.timer_compare;
        let pending = &mut self.csrs.values[csr::INTERRUPT_PENDING as usize];
        if timer_active {
            *pending |= interrupt::TIMER;
        } else {
            *pending &= !interrupt::TIMER;
        }
        if self.uart_rx_control & 1 != 0 && !self.uart_input.is_empty() {
            *pending |= interrupt::UART_RX;
        } else {
            *pending &= !interrupt::UART_RX;
        }
    }

    fn pending_interrupt(&self) -> Option<(u32, u32)> {
        let status_value = self.csrs.values[csr::STATUS as usize];
        if status_value & status::IE == 0 {
            return None;
        }
        let active = self.csrs.values[csr::INTERRUPT_PENDING as usize]
            & self.csrs.values[csr::INTERRUPT_ENABLE as usize];
        if active & interrupt::TIMER != 0 {
            Some((interrupt::TIMER, cause::TIMER_INTERRUPT))
        } else if active & interrupt::UART_RX != 0 {
            Some((interrupt::UART_RX, cause::UART_RX_INTERRUPT))
        } else if active & interrupt::SOFTWARE != 0 {
            Some((interrupt::SOFTWARE, cause::SOFTWARE_INTERRUPT))
        } else {
            None
        }
    }

    fn enter_trap(&mut self, trap_cause: u32, epc: u32, badaddr: u32) {
        let old = self.csrs.values[csr::STATUS as usize];
        let mut next =
            old & !(status::IE | status::PIE | status::PRIVILEGED | status::PREVIOUS_PRIVILEGED);
        if old & status::IE != 0 {
            next |= status::PIE;
        }
        if old & status::PRIVILEGED != 0 {
            next |= status::PREVIOUS_PRIVILEGED;
        }
        next |= status::PRIVILEGED;
        self.csrs.values[csr::STATUS as usize] = next;
        self.csrs.values[csr::EPC as usize] = epc;
        self.csrs.values[csr::CAUSE as usize] = trap_cause;
        self.csrs.values[csr::BADADDR as usize] = badaddr;
        self.pc = self.csrs.values[csr::TVEC as usize];
        self.waiting_for_interrupt = false;
        if self.trace_interrupts || self.trace_syscalls && trap_cause == cause::ECALL {
            eprintln!("trap cause={trap_cause} epc=0x{epc:08x} badaddr=0x{badaddr:08x}");
        }
    }

    fn trap_or_fault(
        &mut self,
        trap_cause: u32,
        epc: u32,
        badaddr: u32,
        fault: Fault,
    ) -> Result<Option<StopReason>, Fault> {
        if self.csrs.values[csr::TVEC as usize] == 0 {
            Err(fault)
        } else {
            self.enter_trap(trap_cause, epc, badaddr);
            Ok(None)
        }
    }

    fn privileged_fault(
        &mut self,
        instruction_pc: u32,
        word: u32,
    ) -> Result<Option<StopReason>, Fault> {
        self.trap_or_fault(
            cause::PRIVILEGED_INSTRUCTION,
            instruction_pc,
            word,
            Fault::IllegalInstruction {
                pc: instruction_pc,
                word,
            },
        )
    }

    pub fn step(&mut self) -> Result<Option<StopReason>, Fault> {
        use opcode::*;
        self.tick();
        self.steps = self.steps.wrapping_add(1);

        if let Some((source, trap_cause)) = self.pending_interrupt() {
            if source == interrupt::SOFTWARE {
                self.csrs.values[csr::INTERRUPT_PENDING as usize] &= !interrupt::SOFTWARE;
            }
            self.enter_trap(trap_cause, self.pc, 0);
            return Ok(None);
        }
        if self.waiting_for_interrupt {
            return Ok(None);
        }

        let instruction_pc = self.pc;
        let word = match self.fetch() {
            Ok(word) => word,
            Err((trap_cause, fault)) => {
                return self.trap_or_fault(trap_cause, instruction_pc, instruction_pc, fault);
            }
        };
        let instruction = match decode(word, instruction_pc) {
            Ok(instruction) => instruction,
            Err(fault) => {
                return self.trap_or_fault(cause::ILLEGAL_INSTRUCTION, instruction_pc, word, fault);
            }
        };
        if self.trace {
            eprintln!("PC={instruction_pc:08x} IR={word:08x} {instruction:?}");
        }
        self.pc = self.pc.wrapping_add(4);

        match instruction {
            Instruction::Nop => {}
            Instruction::R { op, rd, rs1, rs2 } => {
                let x = self.registers[rs1];
                let y = self.registers[rs2];
                let value = match op {
                    ADD => x.wrapping_add(y),
                    SUB => x.wrapping_sub(y),
                    AND => x & y,
                    OR => x | y,
                    XOR => x ^ y,
                    SHL => x.wrapping_shl(y & 31),
                    SHR => x.wrapping_shr(y & 31),
                    SAR => ((x as i32) >> (y & 31)) as u32,
                    _ => unreachable!(),
                };
                self.set_reg(rd, value);
            }
            Instruction::Movi { rd, imm } => self.set_reg(rd, imm as u32),
            Instruction::Addi { rd, rs1, imm } => {
                self.set_reg(rd, self.registers[rs1].wrapping_add(imm as u32));
            }
            Instruction::Lui { rd, imm } => self.set_reg(rd, (imm as u32) << 16),
            Instruction::Memory {
                op,
                reg,
                base,
                offset,
            } => {
                let address = self.registers[base].wrapping_add(offset as u32);
                let result = match op {
                    LOAD => self.load_word(address).map(|value| {
                        self.set_reg(reg, value);
                        None
                    }),
                    LOADB => self.load_byte(address).map(|value| {
                        self.set_reg(reg, value);
                        None
                    }),
                    STORE | STOREB => self.store(address, self.registers[reg], op == STOREB),
                    _ => unreachable!(),
                };
                match result {
                    Ok(Some(reason)) => return Ok(Some(reason)),
                    Ok(None) => {}
                    Err(fault) => {
                        let store = op == STORE || op == STOREB;
                        let misaligned = matches!(fault, Fault::UnalignedAccess(_));
                        let trap_cause = match (store, misaligned) {
                            (false, true) => cause::LOAD_MISALIGNED,
                            (false, false) => cause::LOAD_ACCESS,
                            (true, true) => cause::STORE_MISALIGNED,
                            (true, false) => cause::STORE_ACCESS,
                        };
                        return self.trap_or_fault(trap_cause, instruction_pc, address, fault);
                    }
                }
            }
            Instruction::Branch {
                op,
                rs1,
                rs2,
                offset,
            } => {
                let x = self.registers[rs1];
                let y = self.registers[rs2];
                let taken = match op {
                    BEQ => x == y,
                    BNE => x != y,
                    BLT => (x as i32) < (y as i32),
                    BGE => (x as i32) >= (y as i32),
                    _ => unreachable!(),
                };
                if taken {
                    self.pc = self.pc.wrapping_add((offset as u32) << 2);
                }
            }
            Instruction::Jump { call, offset } => {
                if call {
                    self.set_reg(14, self.pc);
                }
                self.pc = self.pc.wrapping_add((offset as u32) << 2);
            }
            Instruction::Ret => self.pc = self.registers[14],
            Instruction::Halt => return Ok(Some(StopReason::Halt)),
            Instruction::CsrRead { rd, csr: number } => {
                if !self.kernel_mode()
                    && !matches!(number, csr::TIMER_COUNT_LO | csr::TIMER_COUNT_HI)
                {
                    return self.privileged_fault(instruction_pc, word);
                }
                let Some(value) = self.csrs.read(number) else {
                    return self.trap_or_fault(
                        cause::ILLEGAL_INSTRUCTION,
                        instruction_pc,
                        word,
                        Fault::IllegalInstruction {
                            pc: instruction_pc,
                            word,
                        },
                    );
                };
                self.set_reg(rd, value);
            }
            Instruction::CsrWrite { rs, csr: number } => {
                if !self.kernel_mode() {
                    return self.privileged_fault(instruction_pc, word);
                }
                if !self.csrs.write(number, self.registers[rs]) {
                    return self.trap_or_fault(
                        cause::ILLEGAL_INSTRUCTION,
                        instruction_pc,
                        word,
                        Fault::IllegalInstruction {
                            pc: instruction_pc,
                            word,
                        },
                    );
                }
            }
            Instruction::Eret => {
                if !self.kernel_mode() {
                    return self.privileged_fault(instruction_pc, word);
                }
                let old = self.csrs.values[csr::STATUS as usize];
                let mut next = old
                    & !(status::IE
                        | status::PIE
                        | status::PRIVILEGED
                        | status::PREVIOUS_PRIVILEGED);
                if old & status::PIE != 0 {
                    next |= status::IE;
                }
                if old & status::PREVIOUS_PRIVILEGED != 0 {
                    next |= status::PRIVILEGED;
                }
                next |= status::PREVIOUS_PRIVILEGED;
                self.csrs.values[csr::STATUS as usize] = next;
                self.pc = self.csrs.values[csr::EPC as usize];
            }
            Instruction::Ecall => {
                if self.trace_syscalls {
                    eprintln!(
                        "ecall number={} pc=0x{instruction_pc:08x}",
                        self.registers[1]
                    );
                }
                return self.trap_or_fault(
                    cause::ECALL,
                    self.pc,
                    0,
                    Fault::IllegalInstruction {
                        pc: instruction_pc,
                        word,
                    },
                );
            }
            Instruction::Wfi => self.waiting_for_interrupt = true,
        }
        self.registers[0] = 0;
        self.update_pending();
        Ok(None)
    }

    pub fn run(&mut self, max_steps: u64) -> Result<StopReason, Fault> {
        while self.steps < max_steps {
            if let Some(reason) = self.step()? {
                return Ok(reason);
            }
        }
        Err(Fault::StepLimit(max_steps))
    }

    pub fn register_dump(&self) -> String {
        (0..16)
            .map(|i| {
                format!(
                    "r{i:02}=0x{:08x}{}",
                    self.registers[i],
                    if i % 4 == 3 { "\n" } else { "  " }
                )
            })
            .collect()
    }

    pub fn csr_dump(&self) -> String {
        (0..csr::COUNT)
            .map(|i| {
                format!(
                    "csr{i:02x}=0x{:08x}\n",
                    self.csrs.read(i as u8).unwrap_or(0)
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn r(op: u8, rd: u32, a: u32, b: u32) -> u32 {
        (op as u32) << 26 | rd << 22 | a << 18 | b << 14
    }

    fn movi(rd: u32, value: i32) -> u32 {
        (opcode::MOVI as u32) << 26 | rd << 22 | (value as u32 & 0x3f_ffff)
    }

    fn csrr(rd: u32, number: u8) -> u32 {
        (opcode::CSRR as u32) << 26 | rd << 22 | number as u32
    }

    fn csrw(number: u8, rs: u32) -> u32 {
        (opcode::CSRW as u32) << 26 | rs << 22 | number as u32
    }

    #[test]
    fn 算術論理とr0固定を実行する() {
        use opcode::*;
        let code = image(&[
            movi(1, 40),
            movi(2, 2),
            r(ADD, 3, 1, 2),
            r(SUB, 4, 1, 2),
            r(AND, 5, 1, 2),
            r(OR, 6, 1, 2),
            r(XOR, 7, 1, 2),
            r(ADD, 0, 1, 2),
            (HALT as u32) << 26,
        ]);
        let mut cpu = Cpu::new(&code, false).unwrap();
        assert_eq!(cpu.run(20), Ok(StopReason::Halt));
        assert_eq!(
            (cpu.registers[3], cpu.registers[4], cpu.registers[5]),
            (42, 38, 0)
        );
        assert_eq!(
            (cpu.registers[6], cpu.registers[7], cpu.registers[0]),
            (42, 42, 0)
        );
    }

    #[test]
    fn シフトの符号を区別する() {
        use opcode::*;
        let code = image(&[
            movi(1, -8),
            movi(2, 2),
            r(SHL, 3, 1, 2),
            r(SHR, 4, 1, 2),
            r(SAR, 5, 1, 2),
            (HALT as u32) << 26,
        ]);
        let mut cpu = Cpu::new(&code, false).unwrap();
        cpu.run(10).unwrap();
        assert_eq!(cpu.registers[3], 0xffff_ffe0);
        assert_eq!(cpu.registers[4], 0x3fff_fffe);
        assert_eq!(cpu.registers[5], 0xffff_fffe);
    }

    #[test]
    fn tvec未設定では従来faultを返す() {
        assert!(matches!(
            Cpu::new(&image(&[1]), false).unwrap().step(),
            Err(Fault::IllegalInstruction { .. })
        ));
        let code = image(&[
            movi(1, RAM_BASE as i32 + 1),
            (opcode::LOAD as u32) << 26 | 2 << 22 | 1 << 18,
        ]);
        let mut cpu = Cpu::new(&code, false).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.step(), Err(Fault::UnalignedAccess(RAM_BASE + 1)));
    }

    #[test]
    fn ecallを処理してeretで次命令へ戻る() {
        use opcode::*;
        let mut words = vec![
            movi(1, 0x20),
            csrw(csr::TVEC, 1),
            (ECALL as u32) << 26,
            movi(2, 42),
            (HALT as u32) << 26,
        ];
        words.resize(8, 0);
        words.extend([csrr(3, csr::CAUSE), csrr(4, csr::EPC), (ERET as u32) << 26]);
        let mut cpu = Cpu::new(&image(&words), false).unwrap();
        assert_eq!(cpu.run(30), Ok(StopReason::Halt));
        assert_eq!(cpu.registers[2], 42);
        assert_eq!(cpu.registers[3], cause::ECALL);
        assert_eq!(cpu.registers[4], 12);
    }

    #[test]
    fn timer_pendingを保持してwfiから復帰する() {
        use opcode::*;
        let mut words = vec![
            movi(1, 0x40),
            csrw(csr::TVEC, 1),
            movi(1, 20),
            csrw(csr::TIMER_COMPARE_LO, 1),
            movi(1, 0),
            csrw(csr::TIMER_COMPARE_HI, 1),
            movi(1, 1),
            csrw(csr::TIMER_CONTROL, 1),
            csrw(csr::INTERRUPT_ENABLE, 1),
            movi(1, 5),
            csrw(csr::STATUS, 1),
            (WFI as u32) << 26,
            movi(2, 77),
            (HALT as u32) << 26,
        ];
        words.resize(16, 0);
        words.extend([
            csrr(3, csr::CAUSE),
            movi(1, -1),
            csrw(csr::TIMER_COMPARE_LO, 1),
            csrw(csr::TIMER_COMPARE_HI, 1),
            (ERET as u32) << 26,
        ]);
        let mut cpu = Cpu::new(&image(&words), false).unwrap();
        assert_eq!(cpu.run(100), Ok(StopReason::Halt));
        assert_eq!(cpu.registers[2], 77);
        assert_eq!(cpu.registers[3], cause::TIMER_INTERRUPT);
    }

    #[test]
    fn uart受信fifoと割り込みを処理する() {
        let code = image(&[(opcode::HALT as u32) << 26]);
        let mut cpu = Cpu::new(&code, false).unwrap();
        cpu.inject_uart_input(b"abc");
        assert_eq!(cpu.load_word(UART_RX_STATUS).unwrap() & 1, 1);
        assert_eq!(cpu.load_byte(UART_RX_DATA).unwrap(), b'a' as u32);
        assert_eq!(cpu.load_byte(UART_RX_DATA).unwrap(), b'b' as u32);
        assert_eq!(cpu.load_byte(UART_RX_DATA).unwrap(), b'c' as u32);
        assert_eq!(cpu.load_byte(UART_RX_DATA).unwrap(), 0);
    }

    #[test]
    fn user_modeの特権命令をprecise例外にする() {
        use opcode::*;
        let mut words = vec![0; 16];
        words[8] = csrw(csr::STATUS, 1);
        words[12] = csrr(3, csr::CAUSE);
        words[13] = csrr(4, csr::EPC);
        words[14] = (HALT as u32) << 26;
        let mut cpu = Cpu::new(&image(&words), false).unwrap();
        cpu.pc = 0x20;
        cpu.set_csr_for_test(csr::TVEC, 0x30);
        cpu.set_csr_for_test(csr::USER_BASE, 0x20);
        cpu.set_csr_for_test(csr::USER_LIMIT, 0x30);
        cpu.set_csr_for_test(csr::STATUS, 0);
        assert_eq!(cpu.run(20), Ok(StopReason::Halt));
        assert_eq!(cpu.registers[3], cause::PRIVILEGED_INSTRUCTION);
        assert_eq!(cpu.registers[4], 0x20);
    }

    #[test]
    fn user_modeのmmio書込みを拒否する() {
        use opcode::*;
        let mut words = vec![0; 16];
        words[8] = (STOREB as u32) << 26 | 2 << 22 | 1 << 18;
        words[12] = csrr(3, csr::CAUSE);
        words[13] = csrr(4, csr::BADADDR);
        words[14] = (HALT as u32) << 26;
        let mut cpu = Cpu::new(&image(&words), false).unwrap();
        cpu.pc = 0x20;
        cpu.registers[1] = UART_TX;
        cpu.registers[2] = b'X' as u32;
        cpu.set_csr_for_test(csr::TVEC, 0x30);
        cpu.set_csr_for_test(csr::USER_BASE, 0x20);
        cpu.set_csr_for_test(csr::USER_LIMIT, 0x30);
        cpu.set_csr_for_test(csr::STATUS, 0);
        assert_eq!(cpu.run(20), Ok(StopReason::Halt));
        assert_eq!(cpu.registers[3], cause::STORE_ACCESS);
        assert_eq!(cpu.registers[4], UART_TX);
        assert!(cpu.uart_output.is_empty());
    }
}
