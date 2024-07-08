use std::collections::HashMap;
use crate::opcodes;
use crate::opcodes::AddressingMode;

const MEM_SIZE: usize = 0x10000;
const PRG_REF: u16 = 0xFFFC;
const PRG_START: u16 = 0x8000;
const STACK_START: u8 = 0x00FF;
const STACK_END: u16 = 0x0100;

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


pub trait Mem {
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
            stack_ptr: STACK_START,
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

    fn push_stack(&mut self, val: u8) {
        self.mem_write(STACK_END | self.stack_ptr as u16, val);
        self.set_stack_ptr(self.stack_ptr.wrapping_sub(1));
    }

    fn push_stack_u16(&mut self, val: u16) {
        self.push_stack((val >> 8) as u8);
        self.push_stack(val as u8);
    }

    fn pull_stack(&mut self) -> u8 {
        self.set_stack_ptr(self.stack_ptr.wrapping_add(1));
        return self.mem_read(STACK_END | self.stack_ptr as u16);
    }

    fn pull_stack_u16(&mut self) -> u16 {
        let res: u16 = self.pull_stack() as u16;
        return res | ((self.pull_stack() as u16) << 8);
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

    fn set_acc(&mut self, new_val: u8) {
        self.accumulator = new_val;
        self.set_zero_and_neg_flags(new_val);
    }

    fn set_reg_x(&mut self, new_val: u8) {
        self.register_x = new_val;
        self.set_zero_and_neg_flags(new_val);
    }

    fn set_reg_y(&mut self, new_val: u8) {
        self.register_y = new_val;
        self.set_zero_and_neg_flags(new_val);
    }

    fn set_stack_ptr(&mut self, new_val: u8) {
        self.stack_ptr = new_val
    }

    fn write_and_set(&mut self, addr: u16, data: u8) {
        self.mem_write(addr, data);
        self.set_zero_and_neg_flags(data);
    }

    fn add_to_acc(&mut self, operand: u8) {
        let sum: u16 = self.accumulator as u16
            + operand as u16
            + (if self.get_flag(F_CARRY) {
                1
            } else {
                0
            }) as u16;
        if sum > 0xFF {
            self.set_flag(F_CARRY);
        } else {
            self.clear_flag(F_CARRY);
        }
        let result: u8 = sum as u8;

        if (operand ^ result) & (self.accumulator ^ result) & 0x80 != 0 {
            self.set_flag(F_OVERFLOW);
        } else {
            self.clear_flag(F_OVERFLOW);
        }
        self.set_acc(result);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.add_to_acc(operand);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator & operand);
    }

    fn asl_acc(&mut self) {
        let initial_val: u8 = self.accumulator;
        if initial_val & 0b1000_0000 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        self.set_acc(self.accumulator << 1);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val = self.mem_read(addr);
        let final_val = initial_val << 1;

        if initial_val & 0b1000_0000 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        self.write_and_set(addr, final_val);
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

    fn branch_on_set(&mut self, flag: u8) {
        if !self.get_flag(flag) {
            return;
        }
        let operand: i8 = self.mem_read(self.program_counter) as i8;
        self.program_counter = self.program_counter
                                    .wrapping_add(1)
                                    .wrapping_add(operand as u16);
    }

    fn branch_on_clear(&mut self, flag: u8) {
        if self.get_flag(flag) {
            return;
        }
        let operand: i8 = self.mem_read(self.program_counter) as i8;
        self.program_counter = self.program_counter
                                    .wrapping_add(1)
                                    .wrapping_add(operand as u16);
    }

    fn cmp(&mut self, mode: &AddressingMode, register: u8) {
        let addr: u16 = self.get_operand_address(mode);
        let val: i8 = self.mem_read(addr) as i8;
        if register as i8 > val {
            self.set_flag(F_CARRY);
        }
        else if register as i8 == val {
            self.set_flag(F_CARRY);
            self.set_flag(F_ZERO);
        }
        else {
            self.set_flag(F_NEG);
        }
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.write_and_set(addr, val.wrapping_sub(1));
    }

    fn dex(&mut self) {
        self.set_reg_x(self.register_x.wrapping_sub(1));
    }

    fn dey(&mut self) {
        self.set_reg_y(self.register_y.wrapping_sub(1));
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator ^ operand)
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.write_and_set(addr, val.wrapping_add(1));
    }

    fn inx(&mut self) {
        self.set_reg_x(self.register_x.wrapping_add(1));
    }

    fn iny(&mut self) {
        self.set_reg_y(self.register_y.wrapping_add(1));
    }

    fn jmp(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.program_counter = addr;
    }

    fn jsr(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute);
        self.push_stack_u16(self.program_counter + 1);
        self.program_counter = addr;
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.set_acc(val);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.set_reg_x(val);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        self.set_reg_y(val);
    }

    fn lsr_acc(&mut self) {
        if self.accumulator & 0b0000_0001 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        self.set_acc(self.accumulator >> 1);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let final_val: u8 = initial_val >> 1;

        if initial_val & 0b0000_0001 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        self.write_and_set(addr, final_val);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator | operand);
    }

    fn pla(&mut self) {
        let val: u8 = self.pull_stack();
        self.set_acc(val);
    }

    fn rol_acc(&mut self) {
        let prev_acc: u8 = self.accumulator;
        if self.get_flag(F_CARRY) {
            self.set_acc((self.accumulator << 1) | 0b0000_0001);
        }
        else {
            self.set_acc(self.accumulator << 1);
        }
        if prev_acc & 0b1000_0000 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
    }

    fn rol(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let mut final_val: u8 = initial_val << 1;
        if self.get_flag(F_CARRY) {
            final_val |= 0b0000_0001;
        }

        if initial_val & 0b1000_0000 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }

        self.write_and_set(addr, final_val);
    }

    fn ror_acc(&mut self) {
        let prev_acc: u8 = self.accumulator;

        if self.get_flag(F_CARRY) {
            self.set_acc(self.accumulator >> 1 | 0b1000_0000);
        }
        else {
            self.set_acc(self.accumulator >> 1);
        }

        if prev_acc & 0b0000_0001 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
    }

    fn ror(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let mut final_val: u8 = initial_val >> 1;
        if self.get_flag(F_CARRY) {
            final_val |= 0b1000_0000;
        }
        
        if initial_val & 0b0000_0001 != 0 {
            self.set_flag(F_CARRY);
        }
        else {
            self.clear_flag(F_CARRY);
        }
        
        self.write_and_set(addr, final_val);
    }

    fn rti(&mut self) {
        self.status = self.pull_stack();
        self.program_counter = self.pull_stack_u16();
    }
    
    fn rts(&mut self) {
        self.program_counter = self.pull_stack_u16();
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.add_to_acc((operand as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.mem_write(addr, self.accumulator);
    }
    
    fn stx(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
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
                0x0A => self.asl_acc(),
                0x06 | 0x16 | 0x0E | 0x1E => self.asl(&opcode.mode),
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
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => self.cmp(&opcode.mode, self.accumulator),
                0xE0 | 0xE4 | 0xEC => self.cmp(&opcode.mode, self.register_x),
                0xC0 | 0xC4 | 0xCC => self.cmp(&opcode.mode, self.register_y),
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
                0x4A => self.lsr_acc(),
                0x46 | 0x56 | 0x4E | 0x5E => self.lsr(&opcode.mode),
                0xEA => {},
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x48 => self.push_stack(self.accumulator),
                0x08 => self.push_stack(self.status),
                0x68 => self.pla(),
                0x28 => self.status = self.pull_stack(),
                0x2A => self.rol_acc(),
                0x26 | 0x36 | 0x2E | 0x3E => self.rol(&opcode.mode),
                0x6A => self.ror_acc(),
                0x66 | 0x76 | 0x6E | 0x7E => self.ror(&opcode.mode),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => self.sbc(&opcode.mode),
                0x38 => self.set_flag(F_CARRY),
                0xF8 => self.set_flag(F_DEC),
                0x78 => self.set_flag(F_INT),
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode),
                0xAA => self.set_reg_x(self.accumulator),
                0xA8 => self.set_reg_y(self.accumulator),
                0xBA => self.set_reg_x(self.stack_ptr),
                0x8A => self.set_acc(self.register_x),
                0x9A => self.set_stack_ptr(self.register_x),
                0x98 => self.set_acc(self.register_y),
                _ => todo!(""),
            }

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }

    pub fn load_snake(&mut self, program: Vec<u8>) {
        self.memory[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x0600);
    }

    pub fn run_snake(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            callback(self);
            // Get current operation in program
            let code: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state: u16 = self.program_counter;
            let opcode: &&opcodes::OpCode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

            // Run corresponding operation function
            match code {
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x0A => self.asl_acc(),
                0x06 | 0x16 | 0x0E | 0x1E => self.asl(&opcode.mode),
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
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => self.cmp(&opcode.mode, self.accumulator),
                0xE0 | 0xE4 | 0xEC => self.cmp(&opcode.mode, self.register_x),
                0xC0 | 0xC4 | 0xCC => self.cmp(&opcode.mode, self.register_y),
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
                0x4A => self.lsr_acc(),
                0x46 | 0x56 | 0x4E | 0x5E => self.lsr(&opcode.mode),
                0xEA => {},
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x48 => self.push_stack(self.accumulator),
                0x08 => self.push_stack(self.status),
                0x68 => self.pla(),
                0x28 => self.status = self.pull_stack(),
                0x2A => self.rol_acc(),
                0x26 | 0x36 | 0x2E | 0x3E => self.rol(&opcode.mode),
                0x6A => self.ror_acc(),
                0x66 | 0x76 | 0x6E | 0x7E => self.ror(&opcode.mode),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => self.sbc(&opcode.mode),
                0x38 => self.set_flag(F_CARRY),
                0xF8 => self.set_flag(F_DEC),
                0x78 => self.set_flag(F_INT),
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode),
                0xAA => self.set_reg_x(self.accumulator),
                0xA8 => self.set_reg_y(self.accumulator),
                0xBA => self.set_reg_x(self.stack_ptr),
                0x8A => self.set_acc(self.register_x),
                0x9A => self.set_stack_ptr(self.register_x),
                0x98 => self.set_acc(self.register_y),
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

    #[test]
    fn test_push() {
        let mut cpu: CPU = CPU::new();
        cpu.push_stack(0x05);
        assert_eq!(cpu.stack_ptr, 0xFE);
        assert_eq!(cpu.memory[0x01FF], 0x05);
    }

    #[test]
    fn test_pull() {
        let mut cpu: CPU = CPU::new();
        cpu.stack_ptr = 0xFE;
        cpu.memory[0x01FF] = 0x05;
        let res: u8 = cpu.pull_stack();
        assert_eq!(cpu.stack_ptr, 0xFF);
        assert_eq!(res, 0x05);
    }

    #[test_case(
        F_CARRY, 0, F_CARRY;
        "Sets flag"
    )]
    #[test_case(
        F_CARRY, F_CARRY, F_CARRY;
        "Doesn't clear flag"
    )]
    fn test_set_flag(flag: u8, initial_status: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.status = initial_status;
        cpu.set_flag(flag);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        F_CARRY, 0b1111_1111, !F_CARRY;
        "Test flag cleared"
    )]
    #[test_case(
        F_CARRY, 0b0000_0000, 0b0000_0000;
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

    #[test_case(
        0x05, 0x05, 0, 0x0A, 0;
        "adc no flags"
    )]
    #[test_case(
        0x05, 0x05, F_CARRY, 0x0B, 0;
        "adc carry set"
    )]
    #[test_case(
        0x00, 0x00, 0, 0x00, F_ZERO;
        "adc sets zero"
    )]
    #[test_case(
        0x02, 0xFF, 0, 0x01, F_CARRY;
        "adc sets carry"
    )]
    #[test_case(
        0x80, 0x01, 0, 0x81, F_NEG;
        "adc sets neg"
    )]
    #[test_case(
        0x80, 0x81, 0, 0x01, F_OVERFLOW | F_CARRY;
        "adc sets overflow"
    )]
    #[test_case(
        0x7F, 0x01, 0, 0x80, F_NEG | F_OVERFLOW;
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

    #[test_case(
        0b0000_1001, 0b0000_1010, 0b0000_1000, 0;
        "and no flags"
    )]
    #[test_case(
        0b0000_0001, 0b0000_0010, 0b0000_0000, F_ZERO;
        "and sets zero flag"
    )]
    #[test_case(
        0b1000_0001, 0b1000_0010, 0b1000_0000, F_NEG;
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

    #[test_case(
        0b0000_0001, 0b0000_0010, 0;
        "asl no flags"
    )]
    #[test_case(
        0b0000_0000, 0b0000_0000, F_ZERO;
        "asl sets zero flag"
    )]
    #[test_case(
        0b0100_0000, 0b1000_0000, F_NEG;
        "asl sets neg flag"
    )]
    #[test_case(
        0b1000_0001, 0b0000_0010, F_CARRY;
        "asl sets carry flag"
    )]
    fn test_asl(accumulator: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.asl_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.memory[0x05] = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.asl(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0001, 0b0000_0001, 0;
        "bit no flags"
    )]
    #[test_case(
        0b0000_0001, 0b0000_0010, F_ZERO;
        "bit sets zero flag"
    )]
    #[test_case(
        0b1000_0001, 0b1000_0001, F_NEG;
        "bit sets neg flag"
    )]
    #[test_case(
        0b0100_0001, 0b0100_0001, F_OVERFLOW;
        "bit sets overflow flag"
    )]
    fn test_bit(accumulator: u8, operand: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.memory[0x05] = operand;
        cpu.bit(&AddressingMode::ZeroPage);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x05, F_CARRY, 0x8006;
        "Branches when flag set"
    )]
    #[test_case(
        0x05, 0, 0x8000;
        "No branch when flag clear"
    )]
    #[test_case(
        -0x05, F_CARRY, 0x7FFC;
        "Negative branches"
    )]
    fn test_branch_on_set(offset: i8, initial_status: u8, expected_pc: u16) {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = offset as u8;
        cpu.status = initial_status;
        cpu.branch_on_set(F_CARRY);
        assert_eq!(cpu.program_counter, expected_pc);
    }

    #[test_case(
        0x05, 0, 0x8006;
        "Branches when flag clear"
    )]
    #[test_case(
        0x05, F_CARRY, 0x8000;
        "No branch when flag set"
    )]
    #[test_case(
        -0x05, 0, 0x7FFC;
        "Negative branches"
    )]
    fn test_branch_on_clear(offset: i8, initial_status: u8, expected_pc: u16) {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = offset as u8;
        cpu.status = initial_status;
        cpu.branch_on_clear(F_CARRY);
        assert_eq!(cpu.program_counter, expected_pc);
    }

    #[test]
    fn test_brk() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        assert_eq!(cpu.status, F_BRK);
    }

    #[test_case(
        0x02, 0x01, F_CARRY;
        "cmp greater"
    )]
    #[test_case(
        0x01, 0x01, F_CARRY | F_ZERO;
        "cmp equal"
    )]
    #[test_case(
        0xFF, 0x01, F_NEG;
        "cmp less"
    )]
    fn test_cmp(accumulator: u8, operand: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = operand;
        cpu.cmp(&AddressingMode::Immediate, accumulator);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x02, 0, 0x01, 0;
        "dec no flags"
    )]
    #[test_case(
        0x01, 0, 0x00, F_ZERO;
        "dec sets zero flag"
    )]
    #[test_case(
        0x81, F_NEG, 0x80, F_NEG;
        "dec keeps neg flag"
    )]
    #[test_case(
        0x00, F_ZERO, 0xFF, F_NEG;
        "dec sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, F_NEG, 0x7F, 0;
        "dec clears neg flag"
    )]
    fn test_dec(mem: u8, initial_status: u8, expected_mem: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = 0x05;
        cpu.memory[0x05] = mem;
        cpu.status = initial_status;
        cpu.dec(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_mem);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x02, 0, 0x01, 0;
        "dex no flags"
    )]
    #[test_case(
        0x01, 0, 0x00, F_ZERO;
        "dex sets zero flag"
    )]
    #[test_case(
        0x81, F_NEG, 0x80, F_NEG;
        "dex keeps neg flag"
    )]
    #[test_case(
        0x00, F_ZERO, 0xFF, F_NEG;
        "dex sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, F_NEG, 0x7F, 0;
        "dex clears neg flag"
    )]
    fn test_dex(register_x: u8, initial_status: u8, expected_x: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = register_x;
        cpu.status = initial_status;
        cpu.dex();
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x02, 0, 0x01, 0;
        "dey no flags"
    )]
    #[test_case(
        0x01, 0, 0x00, F_ZERO;
        "dey sets zero flag"
    )]
    #[test_case(
        0x81, F_NEG, 0x80, F_NEG;
        "dey keeps neg flag"
    )]
    #[test_case(
        0x00, F_ZERO, 0xFF, F_NEG;
        "dey sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, F_NEG, 0x7F, 0;
        "dey clears neg flag"
    )]
    fn test_dey(register_y: u8, initial_status: u8, expected_y: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.register_y = register_y;
        cpu.status = initial_status;
        cpu.dey();
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0101, 0b0000_1100, 0b0000_1001, 0;
        "eor no flags"
    )]
    #[test_case(
        0b0000_0100, 0b0000_0100, 0b0000_0000, F_ZERO;
        "eor sets zero flag"
    )]
    #[test_case(
        0b0000_0001, 0b1000_0001, 0b1000_0000, F_NEG;
        "eor sets neg flag"
    )]
    fn test_eor(accumulator: u8, operand: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = operand;
        cpu.eor(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, 0, 0x02, 0;
        "inc no flags"
    )]
    #[test_case(
        0xFF, 0, 0x00, F_ZERO;
        "inc sets zero flag"
    )]
    #[test_case(
        0x7F, 0, 0x80, F_NEG;
        "inc sets neg flag"
    )]
    #[test_case(
        0x80, F_NEG, 0x81, F_NEG;
        "inc keeps neg flag"
    )]
    fn test_inc(mem: u8, initial_status: u8, expected_mem: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = 0x05;
        cpu.memory[0x05] = mem;
        cpu.status = initial_status;
        cpu.inc(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_mem);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, 0, 0x02, 0;
        "inx no flags"
    )]
    #[test_case(
        0xFF, 0, 0x00, F_ZERO;
        "inx sets zero flag"
    )]
    #[test_case(
        0x7F, 0, 0x80, F_NEG;
        "inx sets neg flag"
    )]
    #[test_case(
        0x80, F_NEG, 0x81, F_NEG;
        "inx keeps neg flag"
    )]
    fn test_inx(register_x: u8, initial_status: u8, expected_x: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = register_x;
        cpu.status = initial_status;
        cpu.inx();
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, 0, 0x02, 0;
        "iny no flags"
    )]
    #[test_case(
        0xFF, 0, 0x00, F_ZERO;
        "iny sets zero flag"
    )]
    #[test_case(
        0x7F, 0, 0x80, F_NEG;
        "iny sets neg flag"
    )]
    #[test_case(
        0x80, F_NEG, 0x81, F_NEG;
        "iny keeps neg flag"
    )]
    fn test_iny(register_y: u8, initial_status: u8, expected_y: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.register_y = register_y;
        cpu.status = initial_status;
        cpu.iny();
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_jmp() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = 0x12;
        cpu.memory[0x01] = 0x34;
        cpu.jmp(&AddressingMode::Absolute);
        assert_eq!(cpu.program_counter, 0x3412);
    }

    #[test]
    fn test_jmp_running() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x4C, 0x05, 0x80, 0xA9, 0xAA, 0xA2, 0x11, 0x00]);
        assert_eq!(cpu.register_x, 0x11);
        assert_eq!(cpu.accumulator, 0);
    }

    #[test]
    fn test_jsr() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = 0x1234;
        cpu.memory[0x1234 as usize] = 0x56;
        cpu.memory[0x1235 as usize] = 0x78;
        cpu.jsr();
        assert_eq!(cpu.program_counter, 0x7856);
        assert_eq!(cpu.memory[0x01FF], 0x12);
        assert_eq!(cpu.memory[0x01FE], 0x35);
    }

    #[test]
    fn test_jsr_and_rts() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x2010] = 0xA9;
        cpu.memory[0x2011] = 0x05;
        cpu.memory[0x2012] = 0x60;
        cpu.load_and_run(vec![0x20, 0x10, 0x20, 0x00]);
        assert_eq!(cpu.accumulator, 0x05);
    }

    #[test_case(
        0x01, 0x01, 0;
        "lda no flags"
    )]
    #[test_case(
        0x00, 0x00, F_ZERO;
        "lda sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, F_NEG;
        "lda sets neg flag"
    )]
    fn test_lda(accumulator: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = accumulator;
        cpu.lda(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, 0x01, 0;
        "ldx no flags"
    )]
    #[test_case(
        0x00, 0x00, F_ZERO;
        "ldx sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, F_NEG;
        "ldx sets neg flag"
    )]
    fn test_ldx(register_x: u8, expected_x: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = register_x;
        cpu.ldx(&AddressingMode::Immediate);
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, 0x01, 0;
        "ldy no flags"
    )]
    #[test_case(
        0x00, 0x00, F_ZERO;
        "ldy sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, F_NEG;
        "ldy sets neg flag"
    )]
    fn test_ldy(register_y: u8, expected_y: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x00] = register_y;
        cpu.ldy(&AddressingMode::Immediate);
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0010, 0, 0b0000_0001, 0;
        "lsr no flags"
    )]
    #[test_case(
        0b1000_0000, F_NEG, 0b0100_0000, 0;
        "lsr clears neg flag"
    )]
    #[test_case(
        0b0000_0001, 0, 0b0000_0000, F_ZERO | F_CARRY;
        "lsr sets carry and zero flag"
    )]
    fn test_lsr(accumulator: u8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.lsr_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.memory[0x05] = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.status = initial_status;
        cpu.lsr(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0101, 0b0000_1100, 0, 0b0000_1101, 0;
        "ora no flags"
    )]
    #[test_case(
        0b0000_0000, 0b0000_0000, F_ZERO, 0b0000_0000, F_ZERO;
        "ora keeps zero flag"
    )]
    #[test_case(
        0b0000_0001, 0b1000_0001, 0, 0b1000_0001, F_NEG;
        "ora sets neg flag"
    )]
    fn test_ora(accumulator: u8, operand: u8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = operand;
        cpu.status = initial_status;
        cpu.ora(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x05, 0x05, 0;
        "pla no flags"
    )]
    #[test_case(
        0x00, 0x00, F_ZERO;
        "pla sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, F_NEG;
        "pla sets neg flag"
    )]
    fn test_pla(stack: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.stack_ptr = 0xFE;
        cpu.memory[0x01FF] = stack;
        cpu.pla();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.stack_ptr, 0xFF);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0001, 0, 0b0000_0010, 0;
        "rol no flags"
    )]
    #[test_case(
        0b0000_0001, F_CARRY, 0b0000_0011, 0;
        "rol with carry flag"
    )]
    #[test_case(
        0b1000_0001, F_NEG, 0b0000_0010, F_CARRY;
        "rol sets carry flag"
    )]
    #[test_case(
        0b1000_0000, F_NEG, 0b0000_0000, F_ZERO | F_CARRY;
        "rol sets zero flag"
    )]
    #[test_case(
        0b0100_0000, 0, 0b1000_0000, F_NEG;
        "rol sets neg flag"
    )]
    fn test_rol(accumulator: u8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.rol_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.memory[0x05] = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.status = initial_status;
        cpu.rol(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0010, 0, 0b0000_0001, 0;
        "ror no flags"
    )]
    #[test_case(
        0b0000_0010, F_CARRY, 0b1000_0001, F_NEG;
        "ror with carry flag"
    )]
    #[test_case(
        0b1000_0001, F_NEG, 0b0100_0000, F_CARRY;
        "ror sets carry flag"
    )]
    #[test_case(
        0b0000_0001, 0, 0b0000_0000, F_ZERO | F_CARRY;
        "ror sets zero flag"
    )]
    fn test_ror(accumulator: u8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.ror_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.memory[0x05] = accumulator;
        cpu.memory[0x00] = 0x05;
        cpu.status = initial_status;
        cpu.ror(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_rti() {
        let mut cpu: CPU = CPU::new();
        cpu.stack_ptr = 0xFC;
        cpu.memory[0x01FF] = 0x80;
        cpu.memory[0x01FE] = 0x03;
        cpu.memory[0x01FD] = F_CARRY | F_NEG;
        cpu.rti();
        assert_eq!(cpu.program_counter, 0x8003);
        assert_eq!(cpu.status, F_CARRY | F_NEG);
    }

    #[test]
    fn test_rts() {
        let mut cpu: CPU = CPU::new();
        cpu.stack_ptr = 0xFD;
        cpu.memory[0x01FF] = 0x80;
        cpu.memory[0x01FE] = 0x03;
        cpu.rts();
        assert_eq!(cpu.program_counter, 0x8004);
    }

    #[test_case(
        0x05, 0x01, 0, 0x03, F_CARRY;
        "sbc no flags"
    )]
    #[test_case(
        0x05, 0x01, F_CARRY, 0x04, F_CARRY;
        "sbc carry flag set"
    )]
    #[test_case(
        0x05, 0x05, F_CARRY, 0x00, F_ZERO | F_CARRY;
        "sbc sets zero flag"
    )]
    #[test_case(
        0x05, 0x06, F_CARRY, 0xFF, F_NEG;
        "sbc sets neg flag"
    )]
    #[test_case(
        0x80, 0x01, F_CARRY | F_NEG, 0x7F, F_OVERFLOW | F_CARRY;
        "sbc sets overflow flag"
    )]
    #[test_case(
        0x00, -0x01, F_CARRY, 0x01, 0;
        "sbc subtracts negative"
    )]
    #[test_case(
        0x7F, -0x01, F_CARRY, 0x80, F_OVERFLOW | F_NEG;
        "sbc subtracts negative overflow"
    )]
    fn test_sbc(accumulator: u8, operand: i8, initial_status: u8, expected_acc: u8, expected_status: u8) {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = accumulator;
        cpu.memory[0x00] = operand as u8;
        cpu.status = initial_status;
        cpu.sbc(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_sta() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0x01;
        cpu.memory[0x00] = 0x05;
        cpu.sta(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], 0x01)
    }

    #[test]
    fn test_stx() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x01;
        cpu.memory[0x00] = 0x05;
        cpu.stx(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], 0x01)
    }

    #[test]
    fn test_sty() {
        let mut cpu: CPU = CPU::new();
        cpu.register_y = 0x01;
        cpu.memory[0x00] = 0x05;
        cpu.sty(&AddressingMode::ZeroPage);
        assert_eq!(cpu.memory[0x05], 0x01)
    }
}