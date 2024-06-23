use std::collections::HashMap;
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
            AddressingMode::Relative => {
                let diff: i8 = self.mem_read(self.program_counter) as i8;
                if diff >= 0 {
                    self.program_counter.wrapping_add(diff as u16)
                }
                else {
                    self.program_counter.wrapping_sub((-diff) as u16)
                }
            },
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
    
    fn unset_flag(&mut self, flag: u8) {
        self.status = self.status & !flag;
    }

    fn set_zero_and_neg_flags(&mut self, val: u8) {
        if val == 0 {
            self.set_flag(F_ZERO);
        } else {
            self.unset_flag(F_ZERO);
        }

        if val & 0b1000_0000 != 0 {
            self.set_flag(F_NEG);
        }
        else {
            self.unset_flag(F_NEG);
        }
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);

        self.set_zero_and_neg_flags(self.register_x);
    }

    fn tax(&mut self) {
        self.register_x = self.accumulator;
                    
        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.accumulator = self.mem_read(addr);

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
                0xAA => self.tax(),
                0xE8 => self.inx(),
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => self.lda(&opcode.mode),
                0x00 => {
                    self.status = self.status | F_BRK;
                    return;
                },
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
        cpu.mem_write(PRG_START, 0xA4);
        assert_eq!(cpu.memory[PRG_START as usize], 0xA4);
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

    #[test]
    fn test_get_operand_address_immediate() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        let immediate: u16 = cpu.get_operand_address(&AddressingMode::Immediate);
        assert_eq!(immediate, PRG_START);
    }

    #[test]
    fn test_get_operand_address_relative() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = 1;
        let relative: u16 = cpu.get_operand_address(&AddressingMode::Relative);
        assert_eq!(relative, PRG_START.wrapping_add(1));

        cpu.memory[PRG_START as usize] = (-1 as i8) as u8;
        let relative_neg: u16 = cpu.get_operand_address(&AddressingMode::Relative);
        assert_eq!(relative_neg, PRG_START.wrapping_sub(1));

        cpu.program_counter = (MEM_SIZE - 1) as u16;
        cpu.memory[MEM_SIZE - 1] = 1;
        let relative_over: u16 = cpu.get_operand_address(&AddressingMode::Relative);
        assert_eq!(relative_over, 0);

        cpu.program_counter = 0;
        cpu.memory[0] = (-1 as i8) as u8;
        let relative_under: u16 = cpu.get_operand_address(&AddressingMode::Relative);
        assert_eq!(relative_under, (MEM_SIZE - 1) as u16);
    }

    #[test]
    fn test_get_operand_address_zero_page() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = 1;
        let zero_page: u16 = cpu.get_operand_address(&AddressingMode::ZeroPage);
        assert_eq!(zero_page, 1);

        cpu.register_x = 3;
        let zero_page_x: u16 = cpu.get_operand_address(&AddressingMode::ZeroPage_X);
        assert_eq!(zero_page_x, 4);

        cpu.register_x = 0xFF;
        let zero_page_x_over: u16 = cpu.get_operand_address(&AddressingMode::ZeroPage_X);
        assert_eq!(zero_page_x_over, 0);
        
        cpu.register_y = 4;
        let zero_page_y: u16 = cpu.get_operand_address(&AddressingMode::ZeroPage_Y);
        assert_eq!(zero_page_y, 5);

        cpu.register_y = 0xFF;
        let zero_page_y_over: u16 = cpu.get_operand_address(&AddressingMode::ZeroPage_Y);
        assert_eq!(zero_page_y_over, 0);
    }

    #[test]
    fn test_get_operand_address_absolute() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = 0x01;
        cpu.memory[(PRG_START + 1) as usize] = 0xFF;
        let absolute: u16 = cpu.get_operand_address(&AddressingMode::Absolute);
        assert_eq!(absolute, 0xFF01);
        
        cpu.register_x = 2;
        let absolute_x: u16 = cpu.get_operand_address(&AddressingMode::Absolute_X);
        assert_eq!(absolute_x, 0xFF03);
        
        cpu.register_x = 0xFF;
        let absolute_x_over: u16 = cpu.get_operand_address(&AddressingMode::Absolute_X);
        assert_eq!(absolute_x_over, 0);

        cpu.register_y = 3;
        let absolute_y: u16 = cpu.get_operand_address(&AddressingMode::Absolute_Y);
        assert_eq!(absolute_y, 0xFF04);

        cpu.register_y = 0xFF;
        let absolute_y_over: u16 = cpu.get_operand_address(&AddressingMode::Absolute_Y);
        assert_eq!(absolute_y_over, 0);
    }

    #[test]
    fn test_get_operand_address_indirect() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = 0x01;
        cpu.memory[(PRG_START + 1) as usize] = 0xFF;
        cpu.memory[0xFF01] = 0x10;
        cpu.memory[0xFF02] = 0x20;
        let indirect: u16 = cpu.get_operand_address(&AddressingMode::Indirect);
        assert_eq!(indirect, 0x2010);

        cpu.register_x = 2;
        cpu.memory[0x03] = 0x30;
        cpu.memory[0x04] = 0x40;
        let indirect_x: u16 = cpu.get_operand_address(&AddressingMode::Indirect_X);
        assert_eq!(indirect_x, 0x4030);

        cpu.register_x = 0xFF;
        cpu.memory[0x00] = 0x50;
        cpu.memory[0x01] = 0x60;
        let indirect_x_over: u16 = cpu.get_operand_address(&AddressingMode::Indirect_X);
        assert_eq!(indirect_x_over, 0x6050);

        cpu.register_y = 3;
        cpu.memory[0x01] = 0x70;
        cpu.memory[0x02] = 0x80;
        let indirect_y: u16 = cpu.get_operand_address(&AddressingMode::Indirect_Y);
        assert_eq!(indirect_y, 0x8073);

        cpu.register_y = 0x01;
        cpu.memory[0x01] = 0xFF;
        cpu.memory[0x02] = 0xFF;
        let indirect_y_over: u16 = cpu.get_operand_address(&AddressingMode::Indirect_Y);
        assert_eq!(indirect_y_over, 0x0000);
    }

    #[test]
    fn test_set_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.set_flag(F_CARRY);
        assert_eq!(cpu.status, F_CARRY);
        cpu.set_flag(F_BRK);
        assert_eq!(cpu.status, F_CARRY | F_BRK);
    }

    #[test]
    fn test_unset_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.status = 0b1111_1111;
        cpu.unset_flag(F_CARRY);
        assert_eq!(cpu.status, !F_CARRY);
        cpu.unset_flag(F_BRK);
        assert_eq!(cpu.status, !(F_CARRY | F_BRK));
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

    #[test]
    fn test_brk() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        assert!(cpu.status & F_BRK == F_BRK);
    }

    #[test]
    fn test_lda() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xA9, 0x05, 0x00]);
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0x00, 0x00]);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0xff, 0x00]);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_tax() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xA9, 0x05, 0xAA, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0, 0xAA, 0x00]);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0xff, 0xAA, 0x00]);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_inx() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xA9, 0, 0xAA, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, 0x01);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0xff, 0xAA, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);

        cpu.load_and_run(vec![0xA9, 0x7F, 0xAA, 0xE8, 0x00]);
        assert_eq!(cpu.register_x, 0x80);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }
}