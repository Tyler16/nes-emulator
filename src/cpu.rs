use std::collections::HashMap;
use std::ops::Add;
use crate::opcodes;
use crate::opcodes::AddressingMode;

const MEM_SIZE: usize = 0x10000;
const PRG_REF: u16 = 0xFFFC;
const PRG_START: u16 = 0x8000;

// Flags
const F_NEG: u8 = 0b1000_0000;
const F_OVERFLOW: u8 = 0b0100_0000;
const F_BRK: u8 = 0b0001_0000;
const F_DEC: u8 = 0b0000_1000;
const F_INT: u8 = 0b0000_0100;
const F_ZERO: u8 = 0b0000_0010;
const F_CARRY: u8 = 0b0000_0001;

pub struct CPU {
    pub stack_ptr: u8,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; MEM_SIZE],
}


trait Mem {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);
    
    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        let low: u16 = self.mem_read(addr) as u16;
        let high: u16 = self.mem_read(addr + 1) as u16;
        return (high << 8) | low;
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let high: u8 = (data >> 8) as u8;
        let low: u8 = (data & 0xff) as u8;
        self.mem_write(addr, low);
        self.mem_write(addr + 1, high);
    }
}


impl Mem for CPU {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}


impl CPU {

    pub fn new() -> Self {
        CPU {
            stack_ptr: 0,
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            memory: [0; MEM_SIZE],
        }
    }

    fn get_operand_address(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::ZeroPage_X => {
                let base: u8 = self.mem_read(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_x) as u16;
                addr
            },
            AddressingMode::ZeroPage_Y => {
                let base: u8 = self.mem_read(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_y) as u16;
                addr
            },
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::Absolute_X => {
                let base: u16 = self.mem_read_u16(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_x as u16);
                addr
            },
            AddressingMode::Absolute_Y => {
                let base: u16 = self.mem_read_u16(self.program_counter);
                let addr: u16 = base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::Indirect => {
                let base: u16 = self.mem_read_u16(self.program_counter);
                self.mem_read_u16(base)
            },
            AddressingMode::Indirect_X => {
                let base: u8 = self.mem_read(self.program_counter);
                self.mem_read_u16(base.wrapping_add(self.register_x) as u16)
            }
            AddressingMode::Indirect_Y => {
                let base: u8 = self.mem_read(self.program_counter);
                self.mem_read_u16(base as u16).wrapping_add(self.register_y as u16)
            }
            _ => 0,
        }
    }

    fn set_flag(&mut self, flag: u8) {
        self.status = self.status | flag;
    }
    
    fn clear_flag(&mut self, flag: u8) {
        self.status = self.status & !flag;
    }

    fn get_flag(&mut self, flag: u8) -> bool {
        (self.status & flag) > 0
    }

    fn set_zero_and_neg_flags(&mut self, val: u8) {
        if val == 0 {
            self.set_flag(F_ZERO);
        } else {
            self.clear_flag(F_ZERO);
        }

        if val & 0b1000_0000 != 0 {
            self.set_flag(F_NEG);
        }
        else {
            self.clear_flag(F_NEG);
        }
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        let prev_acc: u8 = self.accumulator;
        let sum: u16 = self.accumulator as u16 + operand as u16 + self.get_flag(F_CARRY) as u16;
        self.accumulator = sum as u8;

        if sum > 0xFF {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY)
        }
        if ((prev_acc ^ self.accumulator) & (operand ^ self.accumulator) & 0b1000_0000) != 0 {
            self.set_flag(F_OVERFLOW);
        }
        else {
            self.clear_flag(F_OVERFLOW);
        }
        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.accumulator = self.mem_read(addr) & self.accumulator;

        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let mut initial_val: u8 = 0;
        let mut final_val: u8 = 0;
        match mode {
            AddressingMode::Accumulator => {
                initial_val = self.accumulator;
                self.accumulator <<= 1;
                final_val = self.accumulator;
            }
            _ => {
                let addr: u16 = self.get_operand_address(mode);
                initial_val = self.mem_read(addr);
                final_val = initial_val << 1;
                self.mem_write(addr, final_val);
            }
        }
        if initial_val & 0b1000_0000 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        self.set_zero_and_neg_flags(final_val);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let mask: u8 = self.mem_read(addr);
        let res: u8 = mask & self.accumulator;
        if res & 0b0100_0000 != 0 {
            self.set_flag(F_OVERFLOW);
        }
        else {
            self.clear_flag(F_OVERFLOW);
        }
        self.set_zero_and_neg_flags(res);
    }

    fn branch_on_set(&mut self, flag: u8) {}

    fn branch_on_clear(&mut self, flag: u8) {}

    fn cmp(&mut self, mode: &AddressingMode) {}

    fn cpx(&mut self, mode: &AddressingMode) {}

    fn cpy(&mut self, mode: &AddressingMode) {}

    fn dec(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.mem_write(addr, val.wrapping_sub(1));
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);

        self.set_zero_and_neg_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);

        self.set_zero_and_neg_flags(self.register_y);
    }

    fn eor(&mut self, mode: &AddressingMode) {}

    fn inc(&mut self, mode: &AddressingMode) {}

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);

        self.set_zero_and_neg_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);

        self.set_zero_and_neg_flags(self.register_y);
    }

    fn jmp(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);

        self.program_counter = addr;
    }

    fn jsr(&mut self) {}

    fn lda(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.accumulator = self.mem_read(addr);

        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.register_x = self.mem_read(addr);

        self.set_zero_and_neg_flags(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.register_y = self.mem_read(addr);

        self.set_zero_and_neg_flags(self.register_y);
    }

    fn lsr(&mut self, mode: &AddressingMode) {}

    fn ora(&mut self, mode: &AddressingMode) {}

    fn pha(&mut self) {}

    fn php(&mut self) {}

    fn pla(&mut self) {}

    fn plp(&mut self) {}

    fn rol(&mut self, mode: &AddressingMode) {}

    fn ror(&mut self, mode: &AddressingMode) {}

    fn rti(&mut self) {}

    fn rts(&mut self) {}

    // todo
    fn sbc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let mut operand: u8 = self.mem_read(addr);
        if operand != 0 {
            operand = !operand + 1;
        }
        let diff: u16 = (self.accumulator as u16 + operand as u16) - (!self.get_flag(F_CARRY) as u16);
        let prev_acc: u8 = self.accumulator;
        self.accumulator = diff as u8;

        if diff > 0xFF {
            self.clear_flag(F_CARRY);
        }
        else {
            self.set_flag(F_CARRY)
        }
        if ((prev_acc ^ self.accumulator) & (operand ^ self.accumulator) & 0b1000_0000) != 0 {
            self.set_flag(F_OVERFLOW);
        }
        else {
            self.clear_flag(F_OVERFLOW);
        }
        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn sta(&mut self, mode: &AddressingMode) {}
    
    fn stx(&mut self, mode: &AddressingMode) {}

    fn sty(&mut self, mode: &AddressingMode) {}

    fn tax(&mut self) {
        self.register_x = self.accumulator;
                    
        self.set_zero_and_neg_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.accumulator;
                    
        self.set_zero_and_neg_flags(self.register_y);
    }

    fn tsx(&mut self) {}

    fn txa(&mut self) {
        self.accumulator = self.register_x;
                    
        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn txs(&mut self) {}

    fn tya(&mut self) {
        self.accumulator = self.register_y;
                    
        self.set_zero_and_neg_flags(self.accumulator);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;

        self.program_counter = self.mem_read_u16(PRG_REF);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[PRG_START as usize .. (PRG_START as usize + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(PRG_REF, PRG_START);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            // Get current operation in program
            let code: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state: u16 = self.program_counter;
            let opcode: &&opcodes::OpCode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

            // Run corresponding operation function
            match code {
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x0A | 0x06 | 0x16 | 0x0E | 0x1E => self.asl(&opcode.mode),
                0x90 => self.branch_on_clear(F_CARRY),
                0xB0 => self.branch_on_set(F_CARRY),
                0xF0 => self.branch_on_set(F_ZERO),
                0x24 | 0x2C => self.bit(&opcode.mode),
                0x30 => self.branch_on_set(F_NEG),
                0xD0 => self.branch_on_clear(F_ZERO),
                0x10 => self.branch_on_clear(F_NEG),
                0x00 => {
                    self.set_flag(F_BRK);
                    return;
                },
                0x50 => self.branch_on_clear(F_OVERFLOW),
                0x70 => self.branch_on_set(F_OVERFLOW),
                0x18 => self.clear_flag(F_CARRY),
                0xD8 => self.clear_flag(F_DEC),
                0x58 => self.clear_flag(F_INT),
                0xB8 => self.clear_flag(F_OVERFLOW),
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => self.cmp(&opcode.mode),
                0xE0 | 0xE4 | 0xEC => self.cpx(&opcode.mode),
                0xC0 | 0xC4 | 0xCC => self.cpy(&opcode.mode),
                0xC6 | 0xD6 | 0xCE | 0xDE => self.dec(&opcode.mode),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode),
                0xE6 | 0xF6 | 0xEE | 0xFE => self.inc(&opcode.mode),
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0x4C | 0x6c => self.jmp(&opcode.mode),
                0x20 => self.jsr(),
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => self.lda(&opcode.mode),
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => self.ldx(&opcode.mode),
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => self.ldy(&opcode.mode),
                0x4A | 0x46 | 0x56 | 0x4E | 0x5E => self.lsr(&opcode.mode),
                0xEA => {},
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0x2A | 0x26 | 0x36 | 0x2E | 0x3E => self.rol(&opcode.mode),
                0x6A | 0x66 | 0x76 | 0x6E | 0x7E => self.ror(&opcode.mode),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => self.sbc(&opcode.mode),
                0x38 => self.set_flag(F_CARRY),
                0xF8 => self.set_flag(F_DEC),
                0x78 => self.set_flag(F_INT),
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode),
                0xAA => self.tax(),
                0xA8 => self.tay(),
                0xBA => self.tsx(),
                0x8A => self.txa(),
                0x9A => self.txs(),
                0x98 => self.tya(),
                _ => todo!(""),
            }

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

    #[test]
    fn test_read_memory() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[PRG_START as usize] = 0xA1;
        assert!(cpu.mem_read(PRG_START) == 0xA1);
    }

    #[test]
    fn test_write_memory() {
        let mut cpu: CPU = CPU::new();
        cpu.mem_write(PRG_START, 0xA1);
        assert_eq!(cpu.memory[PRG_START as usize], 0xA1);
    }

    #[test]
    fn test_read_memory_u16() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[PRG_START as usize] = 0xAA;
        cpu.memory[(PRG_START + 1) as usize] = 0x05;
        assert_eq!(cpu.mem_read_u16(PRG_START), 0x05AA);
    }

    #[test]
    fn test_write_memory_u16() {
        let mut cpu: CPU = CPU::new();
        cpu.mem_write_u16(PRG_START, 0x0508);
        assert_eq!(cpu.memory[PRG_START as usize], 0x08);
        assert_eq!(cpu.memory[(PRG_START + 1) as usize], 0x05);
    }

    #[test_case(
        &AddressingMode::Immediate, 0x00, 0x00, 0x0000, 0x00, 0x00, 0x00, 0x00, PRG_START;
        "Immediate addressing mode"
    )]
    #[test_case(
        &AddressingMode::ZeroPage, 0x01, 0x00, 0x0000, 0x00, 0x00, 0x00, 0x00, 0x0001;
        "Zero page addressing mode"
    )]
    #[test_case(
        &AddressingMode::ZeroPage_X, 0x01, 0x00, 0x0000, 0x00, 0x00, 0x02, 0x00, 0x0003;
        "Zero page X addressing mode"
    )]
    #[test_case(
        &AddressingMode::ZeroPage_X, 0x01, 0x00, 0x0000, 0x00, 0x00, 0xFF, 0x00, 0x0000;
        "Zero page X addressing mode overflow"
    )]
    #[test_case(
        &AddressingMode::ZeroPage_Y, 0x01, 0x00, 0x0000, 0x00, 0x00, 0x00, 0x02, 0x0003;
        "Zero page Y addressing mode"
    )]
    #[test_case(
        &AddressingMode::ZeroPage_Y, 0x01, 0x00, 0x0000, 0x00, 0x00, 0x00, 0xFF, 0x0000;
        "Zero page Y addressing mode overflow"
    )]
    #[test_case(
        &AddressingMode::Absolute, 0x01, 0x02, 0x0000, 0x00, 0x00, 0x00, 0x00, 0x0201;
        "Absolute addressing mode"
    )]
    #[test_case(
        &AddressingMode::Absolute_X, 0x01, 0x02, 0x0000, 0x00, 0x00, 0x03, 0x00, 0x0204;
        "Absolute X addressing mode"
    )]
    #[test_case(
        &AddressingMode::Absolute_X, 0xFF, 0xFF, 0x000, 0x00, 0x00, 0x01, 0x00, 0x0000;
        "Absolute X addressing mode overflow"
    )]
    #[test_case(
        &AddressingMode::Absolute_Y, 0x01, 0x02, 0x0000, 0x00, 0x00, 0x00, 0x03, 0x0204;
        "Absolute Y addressing mode"
    )]
    #[test_case(
        &AddressingMode::Absolute_Y, 0xFF, 0xFF, 0x0000, 0x00, 0x00, 0x00, 0x01, 0x0000;
        "Absolute Y addressing mode overflow"
    )]
    #[test_case(
        &AddressingMode::Indirect, 0x01, 0x02, 0x0201, 0x03, 0x04, 0x00, 0x00, 0x0304;
        "Indirect addressing mode"
    )]
    #[test_case(
        &AddressingMode::Indirect_X, 0x01, 0x00, 0x0021, 0x03, 0x04, 0x20, 0x00, 0x0304;
        "Indirect X addressing mode"
    )]
    #[test_case(
        &AddressingMode::Indirect_X, 0xFF, 0x00, 0x0001, 0x03, 0x04, 0x02, 0x00, 0x0304;
        "Indirect X addressing mode overflow"
    )]
    #[test_case(
        &AddressingMode::Indirect_Y, 0x01, 0x00, 0x0001, 0x03, 0x04, 0x00, 0x10, 0x0314;
        "Indirect Y addressing mode"
    )]
    #[test_case(
        &AddressingMode::Indirect_Y, 0x01, 0x00, 0x0001, 0xFF, 0xFF, 0x00, 0x01, 0x0000;
        "Indirect Y addressing mode overflow"
    )]
    fn test_get_operand_addr(
        mode: &AddressingMode,
        inp1: u8, inp2: u8,
        mem_addr: u16, mem1: u8, mem2: u8,
        register_x: u8, register_y: u8,
        expected: u16
    ) {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.register_x = register_x;
        cpu.register_y = register_y;
        cpu.memory[PRG_START as usize] = inp1;
        cpu.memory[(PRG_START + 1) as usize] = inp2;
        cpu.memory[mem_addr as usize] = mem2;
        cpu.memory[(mem_addr as u16).wrapping_add(1) as usize] = mem1;
        let res: u16 = cpu.get_operand_address(mode);
        assert_eq!(res, expected);
    }

    #[test_case(F_CARRY, 0, F_CARRY;
        "Sets flag"
    )]
    #[test_case(F_CARRY, F_CARRY, F_CARRY;
        "Doesn't clear flag"
    )]
    fn test_set_flag(flag: u8, initial_status: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.status = initial_status;
        cpu.set_flag(flag);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(F_CARRY, 0b1111_1111, !F_CARRY;
        "Test flag cleared"
    )]
    #[test_case(F_CARRY, 0b0000_0000, 0b0000_0000;
        "Test flag remains cleared"
    )]
    fn test_clear_flag(flag: u8, initial_status: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.status = initial_status;
        cpu.clear_flag(flag);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_reset() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0xFF;
        cpu.register_x = 0xFF;
        cpu.register_y = 0xFF;
        cpu.status = 0xFF;
        cpu.mem_write_u16(PRG_REF, PRG_START);
        cpu.reset();
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.status, 0);
        assert_eq!(cpu.program_counter, PRG_START);
    }

    #[test]
    fn test_load() {
        let mut cpu: CPU = CPU::new();
        cpu.load(vec![0xA9, 0x05, 0x00]);
        assert_eq!(cpu.mem_read_u16(PRG_REF), PRG_START);
        assert_eq!(cpu.memory[PRG_START as usize], 0xA9);
        assert_eq!(cpu.memory[(PRG_START + 1) as usize], 0x05);
        assert_eq!(cpu.memory[(PRG_START + 2) as usize], 0x00);
    }

    #[test]
    fn test_run() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = 0xA9;
        cpu.memory[(PRG_START + 1) as usize] = 0x05;
        cpu.memory[(PRG_START + 2) as usize] = 0x00;
        cpu.run();
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.program_counter, PRG_START + 3);
    }

    #[test]
    fn test_load_and_run() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xA9, 0x05, 0x00]);
        assert!(cpu.program_counter == PRG_START + 3);
        assert!(cpu.memory[PRG_START as usize] == 0xA9);
        assert!(cpu.memory[(PRG_START + 1) as usize] == 0x05);
        assert!(cpu.memory[(PRG_START + 2) as usize] == 0x00);
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
    }

    #[test_case(0x05, 0x05, 0, 0x0A, 0;
        "adc no flags"
    )]
    #[test_case(0x05, 0x05, F_CARRY, 0x0B, 0;
        "adc carry set"
    )]
    #[test_case(0x00, 0x00, 0, 0x00, F_ZERO;
        "adc sets zero"
    )]
    #[test_case(0x02, 0xFF, 0, 0x01, F_CARRY;
        "adc sets carry"
    )]
    #[test_case(0x80, 0x01, 0, 0x81, F_NEG;
        "adc sets neg"
    )]
    #[test_case(0x80, 0x81, 0, 0x01, F_OVERFLOW | F_CARRY;
        "adc sets overflow"
    )]
    #[test_case(0x7F, 0x01, 0, 0x80, F_NEG | F_OVERFLOW;
        "adc sets neg and overflow"
    )]
    fn test_adc(accumulator: u8, mem: u8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = mem;
        cpu.status = initial_status;
        cpu.adc(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(0b0000_1001, 0b0000_1010, 0b0000_1000, 0;
        "and with no flags"
    )]
    #[test_case(0b0000_0001, 0b0000_0010, 0b0000_0000, F_ZERO;
        "and sets zero flag"
    )]
    #[test_case(0b1000_0001, 0b1000_0010, 0b1000_0000, F_NEG;
        "and sets neg flag"
    )]
    fn test_and(accumulator: u8, mem: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = mem;
        cpu.and(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status)
    }

    #[test_case(0b0000_0001, 0b0000_0010, 0;
        "asl with no flags"
    )]
    #[test_case(0b0000_0000, 0b0000_0000, F_ZERO;
        "asl sets zero flag"
    )]
    #[test_case(0b0100_0000, 0b1000_0000, F_NEG;
        "asl sets neg flag"
    )]
    #[test_case(0b1000_0001, 0b0000_0010, F_CARRY;
        "asl sets carry flag"
    )]
    fn test_asl(accumulator: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.asl(&AddressingMode::Accumulator);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(0b0000_0001, 0b0000_0001, 0;
        "bit with no flags"
    )]
    #[test_case(0b0000_0001, 0b0000_0010, F_ZERO;
        "bit sets zero flag"
    )]
    #[test_case(0b1000_0001, 0b1000_0001, F_NEG;
        "bit sets neg flag"
    )]
    #[test_case(0b0100_0001, 0b0100_0001, F_OVERFLOW;
        "bit sets overflow flag"
    )]
    fn test_bit(accumulator: u8, memory: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.memory[0x05] = memory;
        cpu.bit(&AddressingMode::ZeroPage);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_brk() {
        let mut cpu: CPU = CPU::new();

        cpu.load_and_run(vec![0x00]);
        assert_eq!(cpu.status, F_BRK);
    }

    #[test]
    fn test_dex() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let test_val: u8 = 0x02;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xCA, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let test_val: u8 = 0xff;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xCA, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_ZERO
        let test_val: u8 = 0x01;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xCA, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test overflow
        let test_val: u8 = 0x00;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xCA, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_NEG changes
        let test_val: u8 = 0x80;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xCA, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_dey() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let test_val: u8 = 0x02;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA0, test_val, 0x88, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let test_val: u8 = 0xff;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA0, test_val, 0x88, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_ZERO
        let test_val: u8 = 0x01;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA0, test_val, 0x88, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test overflow
        let test_val: u8 = 0x00;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA0, test_val, 0x88, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_NEG changes
        let test_val: u8 = 0x80;
        let expected: u8 = test_val.wrapping_sub(1);
        cpu.load_and_run(vec![0xA0, test_val, 0x88, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_inx() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let test_val: u8 = 0;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test overflow and F_ZERO
        let test_val: u8 = 0xff;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let test_val: u8 = 0x7F;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_NEG stays same
        let test_val: u8 = 0x80;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA2, test_val, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_iny() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let test_val: u8 = 0;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA0, test_val, 0xC8, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test overflow and F_ZERO
        let test_val: u8 = 0xff;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA0, test_val, 0xC8, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let test_val: u8 = 0x7F;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA0, test_val, 0xC8, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        // Test F_NEG stays same
        let test_val: u8 = 0x80;
        let expected: u8 = test_val.wrapping_add(1);
        cpu.load_and_run(vec![0xA0, test_val, 0xC8, 0x00]);
        assert_eq!(cpu.register_y, expected);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_jmp_absolute() {
        let mut cpu: CPU = CPU::new();

        cpu.load_and_run(vec![0x4C, 0x05, 0x80, 0xA9, 0xAA, 0xA2, 0x11, 0x00]);
        assert_eq!(cpu.register_x, 0x11);
        assert_eq!(cpu.accumulator, 0);
    }

    #[test]
    fn test_jmp_indirect() {
        let mut cpu: CPU = CPU::new();

        cpu.memory[0x10] = 0x05;
        cpu.memory[0x11] = 0x80;
        cpu.load_and_run(vec![0x6C, 0x10, 0x00, 0xA9, 0xAA, 0xA2, 0x11, 0x00]);
        assert_eq!(cpu.register_x, 0x11);
        assert_eq!(cpu.accumulator, 0);
    }

    #[test]
    fn test_lda_immediate() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        cpu.reset();

        // Test no flags
        cpu.load(vec![0xA9, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        cpu.reset();
        cpu.load(vec![0xA9, 0x00, 0x00]);
        cpu.run();
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        cpu.reset();
        cpu.load(vec![0xA9, 0xff, 0x00]);
        cpu.run();
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_lda_zero_page() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        cpu.reset();

        // Test zero page
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.memory[0x05] = operand;
        cpu.load(vec![0xA5, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);

        // Test zero page X
        let operand: u8 = 0x02;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x06] = operand;
        cpu.register_x = 0x01;
        cpu.load(vec![0xB5, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);
    }

    #[test]
    fn test_lda_absolute() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        cpu.reset();

        // Test absolute
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.memory[0x0505] = operand;
        cpu.load(vec![0xAD, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);

        // Test absolute x
        let operand: u8 = 0x02;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0506] = operand;
        cpu.register_x = 0x01;
        cpu.load(vec![0xBD, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);
        
        // Test absolute y
        let operand: u8 = 0x03;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0507] = operand;
        cpu.register_y = 0x02;
        cpu.load(vec![0xB9, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);
    }

    #[test]
    fn test_lda_indirect() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test indirect x
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.reset();
        cpu.register_x = 0x01;
        cpu.memory[0x06] = 0x05;
        cpu.memory[0x07] = 0x05;
        cpu.memory[0x0505] = operand;
        cpu.load(vec![0xA1, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);

        // Test indirect y
        let operand: u8 = 0x02;
        let result: u8 = operand;
        cpu.reset();
        cpu.register_y = 0x02;
        cpu.memory[0x10] = 0x06;
        cpu.memory[0x11] = 0x06;
        cpu.memory[0x0608] = operand;
        cpu.load(vec![0xB1, 0x10, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, result);
    }

    #[test]
    fn test_ldx_immediate() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA2, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0x00;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA2, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA2, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_ldx_zero_page() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test zero page
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x05] = operand;
        cpu.load(vec![0xA6, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);

        // Test zero page y
        let operand: u8 = 0x02;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x06] = operand;
        cpu.register_y = 0x01;
        cpu.load(vec![0xB6, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
    }

    #[test]
    fn test_ldx_absolute() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test absolute
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0505] = operand;
        cpu.load(vec![0xAE, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
        
        // Test absolute y
        let operand: u8 = 0x03;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0507] = operand;
        cpu.register_y = 0x02;
        cpu.load(vec![0xBE, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_x, result);
    }

    #[test]
    fn test_ldy_immediate() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA0, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0x00;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA0, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.reset();
        cpu.load(vec![0xA0, operand, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_ldy_zero_page() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test zero page
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x05] = operand;
        cpu.load(vec![0xA4, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);

        // Test zero page x
        let operand: u8 = 0x02;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x06] = operand;
        cpu.register_x = 0x01;
        cpu.load(vec![0xB4, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
    }

    #[test]
    fn test_ldy_absolute() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);

        // Test absolute
        let operand: u8 = 0x01;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0505] = operand;
        cpu.load(vec![0xAC, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
        
        // Test absolute x
        let operand: u8 = 0x03;
        let result: u8 = operand;
        cpu.reset();
        cpu.memory[0x0507] = operand;
        cpu.register_x = 0x02;
        cpu.load(vec![0xBC, 0x05, 0x05, 0x00]);
        cpu.run();
        assert_eq!(cpu.register_y, result);
    }

    #[test]
    fn test_set_flag_ops() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        
        cpu.reset();
        cpu.load(vec![0x38, 0x00]);
        cpu.run();
        assert!(cpu.status & F_CARRY == F_CARRY);

        cpu.reset();
        cpu.set_flag(F_CARRY);
        cpu.load(vec![0x38, 0x00]);
        cpu.run();
        assert!(cpu.status & F_CARRY == F_CARRY);

        cpu.reset();
        cpu.load(vec![0xF8, 0x00]);
        cpu.run();
        assert!(cpu.status & F_DEC == F_DEC);

        cpu.reset();
        cpu.set_flag(F_DEC);
        cpu.load(vec![0xF8, 0x00]);
        cpu.run();
        assert!(cpu.status & F_DEC == F_DEC);

        cpu.reset();
        cpu.load(vec![0x78, 0x00]);
        cpu.run();
        assert!(cpu.status & F_INT == F_INT);

        cpu.reset();
        cpu.set_flag(F_INT);
        cpu.load(vec![0x78, 0x00]);
        cpu.run();
        assert!(cpu.status & F_INT == F_INT);
    }

    #[test]
    fn test_sbc_immediate() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        cpu.reset();

        // Test basic sub without carry
        let acc_val: i8 = 5;
        let operand: i8 = 1;
        let res: i8 = 3;
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == 0);

        // Test basic sub with carry
        let acc_val: i8 = 5;
        let operand: i8 = 1;
        let res: i8 = 4;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.set_flag(F_CARRY);
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == 0);

        // Test F_ZERO
        let acc_val: i8 = 5;
        let operand: i8 = 4;
        let res: i8 = 0;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == 0);

        // Test F_ZERO with negatives
        let acc_val: i8 = -2;
        let operand: i8 = -3;
        let res: i8 = 0;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == 0);

        // Test F_NEG
        let acc_val: i8 = -1;
        let operand: i8 = 1;
        let res: i8 = -3;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == 0);

        // Test F_NEG with positive accumulator
        let acc_val: i8 = 1;
        let operand: i8 = 2;
        let res: i8 = -2;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == F_CARRY);

        // Test F_NEG with negative operand
        let acc_val: i8 = -3;
        let operand: i8 = -1;
        let res: i8 = -3;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
        assert!(cpu.status & F_OVERFLOW == 0);
        assert!(cpu.status & F_CARRY == F_CARRY);

        // Test F_OVERFLOW
        let acc_val: i8 = -128;
        let operand: i8 = 1;
        let res: i8 = 126;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
        assert!(cpu.status & F_OVERFLOW == F_OVERFLOW);
        assert!(cpu.status & F_CARRY == 0);

        // Test F_OVERFLOW positive to negative
        let acc_val: i8 = 127;
        let operand: i8 = -2;
        let res: i8 = -128;
        cpu.reset();
        cpu.accumulator = acc_val as u8;
        cpu.load(vec![0xE9, operand as u8, 0x00]);
        cpu.run();
        assert_eq!(cpu.accumulator, res as u8);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
        assert!(cpu.status & F_OVERFLOW == F_OVERFLOW);
        assert!(cpu.status & F_CARRY == F_CARRY);
    }

    #[test]
    fn test_tax() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, operand, 0xAA, 0x00]);
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, 0, 0xAA, 0x00]);
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, 0xFF, 0xAA, 0x00]);
        assert_eq!(cpu.register_x, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_tay() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, operand, 0xA8, 0x00]);
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, 0, 0xA8, 0x00]);
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA9, 0xFF, 0xA8, 0x00]);
        assert_eq!(cpu.register_y, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_txa() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA2, operand, 0x8A, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA2, 0, 0x8A, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA2, 0xFF, 0x8A, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_tya() {
        let mut cpu: CPU = CPU::new();

        // Test no flags
        let operand: u8 = 0x05;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA0, operand, 0x98, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        // Test F_ZERO
        let operand: u8 = 0;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA0, 0, 0x98, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        // Test F_NEG
        let operand: u8 = 0xFF;
        let result: u8 = operand;
        cpu.load_and_run(vec![0xA0, 0xFF, 0x98, 0x00]);
        assert_eq!(cpu.accumulator, result);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }
}