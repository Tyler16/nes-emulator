use std::collections::HashMap;
use crate::opcodes;
use crate::opcodes::AddressingMode;
use crate::bus::Bus;
use crate::mem::Mem;

const PRG_REF: u16 = 0xFFFC;
const PRG_START: u16 = 0x8000;
const STACK_START: u8 = 0x00FD;
const STACK_END: u16 = 0x0100;

bitflags! {
    pub struct CPUFlags: u8 {
        const CARRY     = 0b0000_0001;
        const ZERO      = 0b0000_0010;
        const INT       = 0b0000_0100;
        const DEC       = 0b0000_1000;
        const BRK       = 0b0001_0000;
        const BRK2      = 0b0010_0000;
        const OVER      = 0b0100_0000;
        const NEG       = 0b1000_0000;
    }
}

pub struct CPU {
    pub stack_ptr: u8,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CPUFlags,
    pub program_counter: u16,
    pub bus: Bus,
}


impl Mem for CPU {

    fn mem_read(&self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_read_u16(&self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data);
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data);
    }
}


impl CPU {

    pub fn new(bus: Bus) -> Self {
        CPU {
            stack_ptr: STACK_START,
            accumulator: 0,
            register_x: 0,
            register_y: 0,
            status: CPUFlags::from_bits_truncate(0b0010_0100),
            program_counter: PRG_START,
            bus: bus,
        }
    }

    pub fn get_non_immediate_addr(&self, mode: &AddressingMode, curr_addr: u16) -> u16 {
        match mode {
            AddressingMode::ZeroPage => self.mem_read(curr_addr) as u16,
            AddressingMode::ZeroPage_X => {
                let base: u8 = self.mem_read(curr_addr);
                let addr: u16 = base.wrapping_add(self.register_x) as u16;
                addr
            },
            AddressingMode::ZeroPage_Y => {
                let base: u8 = self.mem_read(curr_addr);
                let addr: u16 = base.wrapping_add(self.register_y) as u16;
                addr
            },
            AddressingMode::Absolute => self.mem_read_u16(curr_addr),
            AddressingMode::Absolute_X => {
                let base: u16 = self.mem_read_u16(curr_addr);
                let addr: u16 = base.wrapping_add(self.register_x as u16);
                addr
            },
            AddressingMode::Absolute_Y => {
                let base: u16 = self.mem_read_u16(curr_addr);
                let addr: u16 = base.wrapping_add(self.register_y as u16);
                addr
            },
            AddressingMode::Indirect => {
                let base: u16 = self.mem_read_u16(curr_addr);
                self.mem_read_u16(base)
            },
            AddressingMode::Indirect_X => {
                let base: u8 = self.mem_read(curr_addr);
                let ptr: u8 = base.wrapping_add(self.register_x);
                let lo: u8 = self.mem_read(ptr as u16);
                let hi: u8 = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base: u8 = self.mem_read(curr_addr);
                let lo: u8 = self.mem_read(base as u16);
                let hi: u8 = self.mem_read(base.wrapping_add(1) as u16);
                let indirect_base: u16 = (hi as u16) << 8 | (lo as u16);
                let res: u16 = indirect_base.wrapping_add(self.register_y as u16);
                res
            }
            _ => 0,
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            _ => self.get_non_immediate_addr(mode, self.program_counter),
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
        self.mem_read(STACK_END | self.stack_ptr as u16)
    }

    fn pull_stack_u16(&mut self) -> u16 {
        let res: u16 = self.pull_stack() as u16;
        res | ((self.pull_stack() as u16) << 8)
    }

    fn set_zero_and_neg_flags(&mut self, val: u8) {
        if val == 0 {
            self.status.insert(CPUFlags::ZERO);
        } else {
            self.status.remove(CPUFlags::ZERO);
        }

        if val & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::NEG);
        }
        else {
            self.status.remove(CPUFlags::NEG);
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
        // Add values as u16
        let sum: u16 = self.accumulator as u16
            + operand as u16
            + (if self.status.contains(CPUFlags::CARRY) {
                1
            } else {
                0
            }) as u16;

        // Check if carry flag needs to be set
        if sum > 0xFF {
            self.status.insert(CPUFlags::CARRY);
        } else {
            self.status.remove(CPUFlags::CARRY);
        }

        // Get final result as u8
        let result: u8 = sum as u8;

        // Check for overflow in last bit
        if (operand ^ result) & (self.accumulator ^ result) & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::OVER);
        } else {
            self.status.remove(CPUFlags::OVER);
        }

        // Update accumulator
        self.set_acc(result);
    }

    fn aac(&mut self) {
        // Get address and operand
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let operand: u8 = self.mem_read(addr);

        // Set flags based on and result
        let check: u8 = self.accumulator & operand;
        self.set_zero_and_neg_flags(check);
        if check & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
    }

    fn adc(&mut self, mode: &AddressingMode) {
        // Add operand to accumulator
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.add_to_acc(operand);
    }

    fn and(&mut self, mode: &AddressingMode) {
        // Set accumulator to bitwise and between itself and operand
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator & operand);
    }

    fn arr(&mut self) {
        // Bitwise and between accumulator and operand
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator & operand);

        // Rotate accumulator
        self.ror_acc();

        // Set flags based on bits 6 and 5 of accumulator
        let bit6: bool = self.accumulator & 0b0010_0000 != 0;
        let bit5: bool = self.accumulator & 0b0001_0000 != 0;
        
        if bit6 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        
        if bit6 ^ bit5 {
            self.status.insert(CPUFlags::OVER);
        }
        else {
            self.status.remove(CPUFlags::OVER);
        }
    }

    fn asl_acc(&mut self) {
        // Bitshift accumulator to left by 1 and set carry to last bit
        let initial_val: u8 = self.accumulator;
        if initial_val & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.set_acc(self.accumulator << 1);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        // Bitshift addressed memory to left by 1 and set carry to last bit
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let final_val: u8 = initial_val << 1;

        if initial_val & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.write_and_set(addr, final_val);
    }

    fn asr(&mut self) {
        // And memory with accumulator and bitshift accumulator to right by 1
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator & operand);
        self.lsr_acc();
    }

    fn atx(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let val: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator & val);
        self.set_reg_x(self.accumulator);
    }

    fn axa(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        self.mem_write(addr, self.accumulator & self.register_x & 7);
    }

    fn axs(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let operand: i8 = self.mem_read(addr).wrapping_neg() as i8;
        let new_val: u16 = (self.register_x & self.accumulator) as u16 + operand as u16;
        if new_val > 0xFF {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.set_reg_x(new_val as u8);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let mask: u8 = self.mem_read(addr);
        let res: u8 = mask & self.accumulator;
        if res == 0 {
            self.status.insert(CPUFlags::ZERO);
        } else {
            self.status.remove(CPUFlags::ZERO);
        }

        self.status.set(CPUFlags::NEG, mask & 0b10000000 > 0);
        self.status.set(CPUFlags::OVER, mask & 0b01000000 > 0);
    }

    fn branch(&mut self, condition: bool) {
        if !condition {
            return;
        }
        let operand: i8 = self.mem_read(self.program_counter) as i8;
        self.program_counter = self.program_counter
                                    .wrapping_add(1)
                                    .wrapping_add(operand as u16);
    }

    fn cmp(&mut self, mode: &AddressingMode, register_val: u8) {
        let addr: u16 = self.get_operand_address(mode);
        let val: u8 = self.mem_read(addr);
        if val <= register_val {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }

        self.set_zero_and_neg_flags(register_val.wrapping_sub(val));
    }

    fn dcp(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let mut val: u8 = self.mem_read(addr);
        val = val.wrapping_sub(1);

        self.mem_write(addr, val);

        if val <= self.accumulator {
            self.status.insert(CPUFlags::CARRY);
        }
        self.set_zero_and_neg_flags(self.accumulator.wrapping_sub(val));
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

    fn isc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, operand);
        self.add_to_acc((operand as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    fn jsr(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute);
        self.push_stack_u16(self.program_counter + 1);
        self.program_counter = addr;
    }

    fn lar(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute_Y);
        let operand: u8 = self.mem_read(addr);
        let val: u8 = operand & self.stack_ptr;
        self.mem_write(addr, val);
        self.set_acc(val);
        self.set_reg_x(val);
        self.set_stack_ptr(val);
    }

    fn lax(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(operand);
        self.set_reg_x(operand);
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
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.set_acc(self.accumulator >> 1);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let final_val: u8 = initial_val >> 1;

        if initial_val & 0b0000_0001 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.write_and_set(addr, final_val);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.accumulator | operand);
    }

    fn php(&mut self) {
        let mut flags: CPUFlags = self.status.clone();
        flags.insert(CPUFlags::BRK);
        flags.insert(CPUFlags::BRK2);
        self.push_stack(flags.bits);
    }

    fn pla(&mut self) {
        let val: u8 = self.pull_stack();
        self.set_acc(val);
    }

    fn plp(&mut self) {
        self.status.bits = self.pull_stack();
        self.status.remove(CPUFlags::BRK);
        self.status.insert(CPUFlags::BRK2);
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let operand: u8 = self.rol(mode);
        self.set_acc(operand & self.accumulator);
    }

    fn rra(&mut self, mode: &AddressingMode) {
        let operand: u8 = self.ror(mode);
        self.add_to_acc(operand);
    }

    fn rol_acc(&mut self) {
        let prev_acc: u8 = self.accumulator;
        if self.status.contains(CPUFlags::CARRY) {
            self.set_acc((self.accumulator << 1) | 0b0000_0001);
        }
        else {
            self.set_acc(self.accumulator << 1);
        }
        if prev_acc & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
    }

    fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let mut final_val: u8 = initial_val << 1;
        if self.status.contains(CPUFlags::CARRY) {
            final_val |= 0b0000_0001;
        }

        if initial_val & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }

        self.write_and_set(addr, final_val);
        final_val
    }

    fn ror_acc(&mut self) {
        let prev_acc: u8 = self.accumulator;

        if self.status.contains(CPUFlags::CARRY) {
            self.set_acc(self.accumulator >> 1 | 0b1000_0000);
        }
        else {
            self.set_acc(self.accumulator >> 1);
        }

        if prev_acc & 0b0000_0001 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
    }

    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr: u16 = self.get_operand_address(mode);
        let initial_val: u8 = self.mem_read(addr);
        let mut final_val: u8 = initial_val >> 1;
        if self.status.contains(CPUFlags::CARRY) {
            final_val |= 0b1000_0000;
        }
        
        if initial_val & 0b0000_0001 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        
        self.write_and_set(addr, final_val);
        final_val
    }

    fn rti(&mut self) {
        self.status.bits = self.pull_stack();
        self.status.remove(CPUFlags::BRK);
        self.status.insert(CPUFlags::BRK2);
        self.program_counter = self.pull_stack_u16();
    }
    
    fn rts(&mut self) {
        self.program_counter = self.pull_stack_u16();
        self.program_counter = self.program_counter.wrapping_add(1);
    }

    fn sax(&mut self, mode: &AddressingMode) {
        // Get address
        let addr: u16 = self.get_operand_address(mode);

        // Get val and do & op with register x
        let val: u8 = self.accumulator & self.register_x;

        // Write to memory
        self.mem_write(addr, val);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        self.add_to_acc((operand as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    fn slo(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        if operand & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::CARRY)
        }
        else {
            self.status.remove(CPUFlags::CARRY)
        }
        let new_op: u8 = operand << 1;
        self.mem_write(addr, new_op);
        self.set_acc(new_op | self.accumulator);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        let addr: u16 = self.get_operand_address(mode);
        let operand: u8 = self.mem_read(addr);
        if operand & 1 != 0 {
            self.status.insert(CPUFlags::CARRY);
        }
        else {
            self.status.remove(CPUFlags::CARRY);
        }
        self.mem_write(addr, operand >> 1);
        self.set_acc((operand >> 1) ^ self.accumulator);
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

    fn sxa(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute_Y);
        let high: u8 = (addr >> 8) as u8;
        self.mem_write(addr, (high + 1) & self.register_x)
    }

    fn sya(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute_X);
        let high: u8 = (addr >> 8) as u8;
        self.mem_write(addr, (high + 1) & self.register_y);
    }

    fn xaa(&mut self) {
        let addr: u16 = self.get_operand_address(&AddressingMode::Immediate);
        let operand: u8 = self.mem_read(addr);
        self.set_acc(self.register_x & operand);
    }

    fn xas(&mut self) {
        let new_stack_ptr: u8 = self.accumulator & self.register_x;
        self.set_stack_ptr(new_stack_ptr);
        let addr: u16 = self.get_operand_address(&AddressingMode::Absolute_Y);
        let high: u8 = (addr >> 8) as u8;
        self.mem_write(addr, (high + 1) & self.stack_ptr);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    // Reset CPU values
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = CPUFlags::from_bits_truncate(0b0010_0100);
        self.program_counter = self.mem_read_u16(PRG_REF);
    }

    // Load program into memory
    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(PRG_START + i, program[i as usize]);
        }
        self.mem_write_u16(PRG_REF, PRG_START);
    }

    // Run program from memory
    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn load_snake(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x600 + i, program[i as usize]);
        }
        self.mem_write_u16(0xFFFC, 0x0600);
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
                0x0B | 0x2B => self.aac(),
                0x87 | 0x97 | 0x83 | 0x8F => self.sax(&opcode.mode),
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => self.adc(&opcode.mode),
                0x29 | 0x25 | 0x35 | 0x2D | 0x3D | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                0x6B => self.arr(),
                0x0A => self.asl_acc(),
                0x06 | 0x16 | 0x0E | 0x1E => self.asl(&opcode.mode),
                0x4B => self.asr(),
                0xAB => self.atx(),
                0x9F | 0x93 => self.axa(&opcode.mode),
                0xCB => self.axs(),
                0x90 => self.branch(!self.status.contains(CPUFlags::CARRY)),
                0xB0 => self.branch(self.status.contains(CPUFlags::CARRY)),
                0xF0 => self.branch(self.status.contains(CPUFlags::ZERO)),
                0x24 | 0x2C => self.bit(&opcode.mode),
                0x30 => self.branch(self.status.contains(CPUFlags::NEG)),
                0xD0 => self.branch(!self.status.contains(CPUFlags::ZERO)),
                0x10 => self.branch(!self.status.contains(CPUFlags::NEG)),
                0x00 => {
                    self.status.insert(CPUFlags::BRK);
                    return;
                },
                0x50 => self.branch(!self.status.contains(CPUFlags::OVER)),
                0x70 => self.branch(self.status.contains(CPUFlags::OVER)),
                0x18 => self.status.remove(CPUFlags::CARRY),
                0xD8 => self.status.remove(CPUFlags::DEC),
                0x58 => self.status.remove(CPUFlags::INT),
                0xB8 => self.status.remove(CPUFlags::OVER),
                0xC9 | 0xC5 | 0xD5 | 0xCD | 0xDD | 0xD9 | 0xC1 | 0xD1 => self.cmp(&opcode.mode, self.accumulator),
                0xE0 | 0xE4 | 0xEC => self.cmp(&opcode.mode, self.register_x),
                0xC0 | 0xC4 | 0xCC => self.cmp(&opcode.mode, self.register_y),
                0xC7 | 0xD7 | 0xCF | 0xDF | 0xDB | 0xC3 | 0xD3 => self.dcp(&opcode.mode),
                0xC6 | 0xD6 | 0xCE | 0xDE => self.dec(&opcode.mode),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0x04 | 0x14 | 0x34 | 0x44 | 0x54 | 0x64 | 0x74 | 0x80 | 0x82 | 0x89 | 0xC2 | 0xD4 | 0xE2 | 0xF4 => {},
                0xE7 | 0xF7 | 0xEF | 0xFF | 0xFB | 0xE3 | 0xF3 => self.isc(&opcode.mode),
                0x49 | 0x45 | 0x55 | 0x4D | 0x5D | 0x59 | 0x41 | 0x51 => self.eor(&opcode.mode),
                0xE6 | 0xF6 | 0xEE | 0xFE => self.inc(&opcode.mode),
                0xE8 => self.inx(),
                0xC8 => self.iny(),

                // JMP
                0x4C => {
                    let jmp_addr: u16 = self.mem_read_u16(self.program_counter);
                    self.program_counter = jmp_addr;
                },

                // JMP Indirect
                0x6C => {
                    let mem_addr: u16 = self.mem_read_u16(self.program_counter);
                    let jmp_addr: u16 = if mem_addr & 0x00FF == 0x00FF {
                        let lo: u8 = self.mem_read(mem_addr);
                        let hi: u8 = self.mem_read(mem_addr & 0xFF00);
                        (hi as u16) << 8 | (lo as u16)
                    } else {
                        self.mem_read_u16(mem_addr)
                    };
                    self.program_counter = jmp_addr;
                }
                0x20 => self.jsr(),
                0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => return,
                0xBB => self.lar(),
                0xA7 | 0xB7 | 0xAF | 0xBF | 0xA3 | 0xB3 => self.lax(&opcode.mode),
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => self.lda(&opcode.mode),
                0xA2 | 0xA6 | 0xB6 | 0xAE | 0xBE => self.ldx(&opcode.mode),
                0xA0 | 0xA4 | 0xB4 | 0xAC | 0xBC => self.ldy(&opcode.mode),
                0x4A => self.lsr_acc(),
                0x46 | 0x56 | 0x4E | 0x5E => self.lsr(&opcode.mode),
                0xEA | 0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {},
                0x09 | 0x05 | 0x15 | 0x0D | 0x1D | 0x19 | 0x01 | 0x11 => self.ora(&opcode.mode),
                0x48 => self.push_stack(self.accumulator),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0x27 | 0x37 | 0x2F | 0x3F | 0x3B | 0x23 | 0x33 => self.rla(&opcode.mode),
                0x67 | 0x77 | 0x6F | 0x7F | 0x7B | 0x63 | 0x73 => self.rra(&opcode.mode),
                0x2A => self.rol_acc(),
                0x26 | 0x36 | 0x2E | 0x3E => {
                    self.rol(&opcode.mode);
                },
                0x6A => self.ror_acc(),
                0x66 | 0x76 | 0x6E | 0x7E => {
                    self.ror(&opcode.mode);
                },
                0x40 => self.rti(),
                0x60 => self.rts(),
                0xEB | 0xE9 | 0xE5 | 0xF5 | 0xED | 0xFD | 0xF9 | 0xE1 | 0xF1 => self.sbc(&opcode.mode),
                0x38 => self.status.insert(CPUFlags::CARRY),
                0xF8 => self.status.insert(CPUFlags::DEC),
                0x78 => self.status.insert(CPUFlags::INT),
                0x07 | 0x17 | 0x0F | 0x1F | 0x1B | 0x03 | 0x13 => self.slo(&opcode.mode),
                0x47 | 0x57 | 0x4F | 0x5F | 0x5B | 0x43 | 0x53 => self.sre(&opcode.mode),
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                0x86 | 0x96 | 0x8E => self.stx(&opcode.mode),
                0x84 | 0x94 | 0x8C => self.sty(&opcode.mode),
                0x9E => self.sxa(),
                0x9C => self.sya(),
                0xAA => self.set_reg_x(self.accumulator),
                0xA8 => self.set_reg_y(self.accumulator),
                0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {},
                0xBA => self.set_reg_x(self.stack_ptr),
                0x8A => self.set_acc(self.register_x),
                0x9A => self.set_stack_ptr(self.register_x),
                0x98 => self.set_acc(self.register_y),
                0x8B => self.xaa(),
                0x9B => self.xas(),
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
    use crate::rom::test;
    use test_case::test_case;

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
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.program_counter = PRG_START;
        cpu.register_x = register_x;
        cpu.register_y = register_y;
        cpu.mem_write(PRG_START, inp1);
        cpu.mem_write(PRG_START + 2, inp2);
        cpu.mem_write(mem_addr, mem2);
        cpu.mem_write(mem_addr.wrapping_add(1), mem1);
        let res: u16 = cpu.get_operand_address(mode);
        assert_eq!(res, expected);
    }

    #[test]
    fn test_push() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.push_stack(0x05);
        assert_eq!(cpu.stack_ptr, 0xFE);
        assert_eq!(cpu.mem_read(0x01FF), 0x05);
    }

    #[test]
    fn test_push_u16() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.push_stack_u16(0x0102);
        assert_eq!(cpu.stack_ptr, 0xFD);
        assert_eq!(cpu.mem_read(0x01FF), 0x01);
        assert_eq!(cpu.mem_read(0x01FE), 0x02);
    }

    #[test]
    fn test_pull() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.stack_ptr = 0xFE;
        cpu.mem_write(0x01FF, 0x05);
        let res: u8 = cpu.pull_stack();
        assert_eq!(cpu.stack_ptr, 0xFF);
        assert_eq!(res, 0x05);
    }

    #[test]
    fn test_pull_u16() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.stack_ptr = 0xFD;
        cpu.mem_write(0x01FE, 0x02);
        cpu.mem_write(0x01FF, 0x01);
        let res: u16 = cpu.pull_stack_u16();
        assert_eq!(cpu.stack_ptr, 0xFF);
        assert_eq!(res, 0x0102);
    }

    #[test]
    fn test_reset() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = 0xFF;
        cpu.register_x = 0xFF;
        cpu.register_y = 0xFF;
        cpu.status.bits = 0xFF;
        cpu.mem_write_u16(PRG_REF, PRG_START);
        cpu.reset();
        assert_eq!(cpu.accumulator, 0);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100));
        assert_eq!(cpu.program_counter, PRG_START);
    }

    #[test]
    fn test_load() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.load(vec![0xA9, 0x05, 0x00]);
        assert_eq!(cpu.mem_read_u16(PRG_REF), PRG_START);
        assert_eq!(cpu.mem_read(PRG_START), 0xA9);
        assert_eq!(cpu.mem_read(PRG_START + 1), 0x05);
        assert_eq!(cpu.mem_read(PRG_START + 2), 0x00);
    }

    #[test]
    fn test_run() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.program_counter = PRG_START;
        cpu.mem_write(PRG_START, 0xA9);
        cpu.mem_write(PRG_START + 1, 0x05);
        cpu.mem_write(PRG_START + 2, 0x00);
        cpu.run();
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.program_counter, PRG_START + 3);
    }

    #[test]
    fn test_load_and_run() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.load_and_run(vec![0xA9, 0x05, 0x00]);
        assert_eq!(cpu.program_counter, PRG_START + 3);
        assert_eq!(cpu.mem_read(PRG_START), 0xA9);
        assert_eq!(cpu.mem_read(PRG_START + 1), 0x05);
        assert_eq!(cpu.mem_read(PRG_START + 2), 0x00);
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.register_x, 0);
        assert_eq!(cpu.register_y, 0);
    }

    #[test_case(
        0x01, CPUFlags::empty();
        "no flags set positive"
    )]
    #[test_case(
        0x00, CPUFlags::ZERO;
        "zero flag set"
    )]
    #[test_case(
        0x80, CPUFlags::NEG;
        "neg flag set"
    )]
    fn test_set_zero_and_neg(val: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.set_zero_and_neg_flags(val);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test]
    fn set_registers() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.set_acc(0x01);
        assert_eq!(cpu.accumulator, 0x01);
        cpu.set_reg_x(0x01);
        assert_eq!(cpu.register_x, 0x01);
        cpu.set_reg_y(0x01);
        assert_eq!(cpu.register_y, 0x01);
        cpu.set_stack_ptr(0x01);
        assert_eq!(cpu.stack_ptr, 0x01);
    }

    #[test_case(
        0x05, 0x05, CPUFlags::empty(), 0x0A, CPUFlags::empty();
        "adc no flags"
    )]
    #[test_case(
        0x05, 0x05, CPUFlags::CARRY, 0x0B, CPUFlags::empty();
        "adc carry set"
    )]
    #[test_case(
        0x00, 0x00, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "adc sets zero"
    )]
    #[test_case(
        0x02, 0xFF, CPUFlags::empty(), 0x01, CPUFlags::CARRY;
        "adc sets carry"
    )]
    #[test_case(
        0x80, 0x01, CPUFlags::empty(), 0x81, CPUFlags::NEG;
        "adc sets neg"
    )]
    #[test_case(
        0x80, 0x81, CPUFlags::empty(), 0x01, CPUFlags::OVER | CPUFlags::CARRY;
        "adc sets overflow"
    )]
    #[test_case(
        0x7F, 0x01, CPUFlags::empty(), 0x80, CPUFlags::NEG | CPUFlags::OVER;
        "adc sets neg and overflow"
    )]
    fn test_adc(accumulator: u8, mem: u8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, mem);
        cpu.status = initial_status;
        cpu.adc(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_1001, 0b0000_1010, 0b0000_1000, CPUFlags::empty();
        "and no flags"
    )]
    #[test_case(
        0b0000_0001, 0b0000_0010, 0b0000_0000, CPUFlags::ZERO;
        "and sets zero flag"
    )]
    #[test_case(
        0b1000_0001, 0b1000_0010, 0b1000_0000, CPUFlags::NEG;
        "and sets neg flag"
    )]
    fn test_and(accumulator: u8, mem: u8, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, mem);
        cpu.and(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status)
    }

    #[test_case(
        0b0000_0001, 0b0000_0010, CPUFlags::empty();
        "asl no flags"
    )]
    #[test_case(
        0b0000_0000, 0b0000_0000, CPUFlags::ZERO;
        "asl sets zero flag"
    )]
    #[test_case(
        0b0100_0000, 0b1000_0000, CPUFlags::NEG;
        "asl sets neg flag"
    )]
    #[test_case(
        0b1000_0001, 0b0000_0010, CPUFlags::CARRY;
        "asl sets carry flag"
    )]
    fn test_asl(accumulator: u8, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.asl_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);

        cpu.mem_write(0x05, accumulator);
        cpu.mem_write(0x00, 0x05);
        cpu.asl(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_acc);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0b0000_0001, 0b0000_0001, CPUFlags::empty();
        "bit no flags"
    )]
    #[test_case(
        0b0000_0001, 0b0000_0010, CPUFlags::ZERO;
        "bit sets zero flag"
    )]
    #[test_case(
        0b1000_0001, 0b1000_0001, CPUFlags::NEG;
        "bit sets neg flag"
    )]
    #[test_case(
        0b0100_0001, 0b0100_0001, CPUFlags::OVER;
        "bit sets overflow flag"
    )]
    fn test_bit(accumulator: u8, operand: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, 0x05);
        cpu.mem_write(0x05, operand);
        cpu.bit(&AddressingMode::ZeroPage);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test]
    fn test_branch() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, 0x05);
        cpu.branch(true);
        assert_eq!(cpu.program_counter, 0x06);
    }

    #[test]
    fn test_brk() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.load_and_run(vec![0x00]);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | CPUFlags::BRK);
    }

    #[test_case(
        0x02, 0x01, CPUFlags::CARRY;
        "cmp greater"
    )]
    #[test_case(
        0x01, 0x01, CPUFlags::CARRY | CPUFlags::ZERO;
        "cmp equal"
    )]
    #[test_case(
        0xFF, 0x01, CPUFlags::NEG;
        "cmp less"
    )]
    fn test_cmp(accumulator: u8, operand: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, operand);
        cpu.cmp(&AddressingMode::Immediate, accumulator);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0x02, CPUFlags::empty(), 0x01, CPUFlags::empty();
        "dec no flags"
    )]
    #[test_case(
        0x01, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "dec sets zero flag"
    )]
    #[test_case(
        0x81, CPUFlags::NEG, 0x80, CPUFlags::NEG;
        "dec keeps neg flag"
    )]
    #[test_case(
        0x00, CPUFlags::ZERO, 0xFF, CPUFlags::NEG;
        "dec sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x7F, CPUFlags::empty();
        "dec clears neg flag"
    )]
    fn test_dec(mem: u8, initial_status: CPUFlags, expected_mem: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, 0x05);
        cpu.mem_write(0x05, mem);
        cpu.status = initial_status;
        cpu.dec(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_mem);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x02, CPUFlags::empty(), 0x01, CPUFlags::empty();
        "dex no flags"
    )]
    #[test_case(
        0x01, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "dex sets zero flag"
    )]
    #[test_case(
        0x81, CPUFlags::NEG, 0x80, CPUFlags::NEG;
        "dex keeps neg flag"
    )]
    #[test_case(
        0x00, CPUFlags::ZERO, 0xFF, CPUFlags::NEG;
        "dex sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x7F, CPUFlags::empty();
        "dex clears neg flag"
    )]
    fn test_dex(register_x: u8, initial_status: CPUFlags, expected_x: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_x = register_x;
        cpu.status = initial_status;
        cpu.dex();
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x02, CPUFlags::empty(), 0x01, CPUFlags::empty();
        "dey no flags"
    )]
    #[test_case(
        0x01, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "dey sets zero flag"
    )]
    #[test_case(
        0x81, CPUFlags::NEG, 0x80, CPUFlags::NEG;
        "dey keeps neg flag"
    )]
    #[test_case(
        0x00, CPUFlags::ZERO, 0xFF, CPUFlags::NEG;
        "dey sets neg and clears zero flag on overflow"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x7F, CPUFlags::empty();
        "dey clears neg flag"
    )]
    fn test_dey(register_y: u8, initial_status: CPUFlags, expected_y: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_y = register_y;
        cpu.status = initial_status;
        cpu.dey();
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0101, 0b0000_1100, 0b0000_1001, CPUFlags::empty();
        "eor no flags"
    )]
    #[test_case(
        0b0000_0100, 0b0000_0100, 0b0000_0000, CPUFlags::ZERO;
        "eor sets zero flag"
    )]
    #[test_case(
        0b0000_0001, 0b1000_0001, 0b1000_0000, CPUFlags::NEG;
        "eor sets neg flag"
    )]
    fn test_eor(accumulator: u8, operand: u8, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, operand);
        cpu.eor(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0x01, CPUFlags::empty(), 0x02, CPUFlags::empty();
        "inc no flags"
    )]
    #[test_case(
        0xFF, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "inc sets zero flag"
    )]
    #[test_case(
        0x7F, CPUFlags::empty(), 0x80, CPUFlags::NEG;
        "inc sets neg flag"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x81, CPUFlags::NEG;
        "inc keeps neg flag"
    )]
    fn test_inc(mem: u8, initial_status: CPUFlags, expected_mem: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, 0x05);
        cpu.mem_write(0x05, mem);
        cpu.status = initial_status;
        cpu.inc(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_mem);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, CPUFlags::empty(), 0x02, CPUFlags::empty();
        "inx no flags"
    )]
    #[test_case(
        0xFF, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "inx sets zero flag"
    )]
    #[test_case(
        0x7F, CPUFlags::empty(), 0x80, CPUFlags::NEG;
        "inx sets neg flag"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x81, CPUFlags::NEG;
        "inx keeps neg flag"
    )]
    fn test_inx(register_x: u8, initial_status: CPUFlags, expected_x: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_x = register_x;
        cpu.status = initial_status;
        cpu.inx();
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x01, CPUFlags::empty(), 0x02, CPUFlags::empty();
        "iny no flags"
    )]
    #[test_case(
        0xFF, CPUFlags::empty(), 0x00, CPUFlags::ZERO;
        "iny sets zero flag"
    )]
    #[test_case(
        0x7F, CPUFlags::empty(), 0x80, CPUFlags::NEG;
        "iny sets neg flag"
    )]
    #[test_case(
        0x80, CPUFlags::NEG, 0x81, CPUFlags::NEG;
        "iny keeps neg flag"
    )]
    fn test_iny(register_y: u8, initial_status: CPUFlags, expected_y: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_y = register_y;
        cpu.status = initial_status;
        cpu.iny();
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_jmp_running() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.load_and_run(vec![0x4C, 0x05, 0x80, 0xA9, 0xAA, 0xA2, 0x11, 0x00]);
        assert_eq!(cpu.register_x, 0x11);
        assert_eq!(cpu.accumulator, 0);
    }

    #[test]
    fn test_jsr() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.program_counter = 0x1234;
        cpu.mem_write(0x1234, 0x56);
        cpu.mem_write(0x1235, 0x78);
        cpu.jsr();
        assert_eq!(cpu.program_counter, 0x7856);
        assert_eq!(cpu.mem_read(0x01FF), 0x12);
        assert_eq!(cpu.mem_read(0x01FE), 0x35);
    }

    #[test]
    fn test_jsr_and_rts() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x2010, 0xA9);
        cpu.mem_write(0x2011, 0x05);
        cpu.mem_write(0x2012, 0x60);
        cpu.load_and_run(vec![0x20, 0x10, 0x20, 0x00]);
        assert_eq!(cpu.accumulator, 0x05);
    }

    #[test_case(
        0x01, 0x01, CPUFlags::empty();
        "lda no flags"
    )]
    #[test_case(
        0x00, 0x00, CPUFlags::ZERO;
        "lda sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, CPUFlags::NEG;
        "lda sets neg flag"
    )]
    fn test_lda(accumulator: u8, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, accumulator);
        cpu.lda(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0x01, 0x01, CPUFlags::empty();
        "ldx no flags"
    )]
    #[test_case(
        0x00, 0x00, CPUFlags::ZERO;
        "ldx sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, CPUFlags::NEG;
        "ldx sets neg flag"
    )]
    fn test_ldx(register_x: u8, expected_x: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, register_x);
        cpu.ldx(&AddressingMode::Immediate);
        assert_eq!(cpu.register_x, expected_x);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0x01, 0x01, CPUFlags::empty();
        "ldy no flags"
    )]
    #[test_case(
        0x00, 0x00, CPUFlags::ZERO;
        "ldy sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, CPUFlags::NEG;
        "ldy sets neg flag"
    )]
    fn test_ldy(register_y: u8, expected_y: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.mem_write(0x00, register_y);
        cpu.ldy(&AddressingMode::Immediate);
        assert_eq!(cpu.register_y, expected_y);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0b0000_0010, CPUFlags::empty(), 0b0000_0001, CPUFlags::empty();
        "lsr no flags"
    )]
    #[test_case(
        0b1000_0000, CPUFlags::NEG, 0b0100_0000, CPUFlags::empty();
        "lsr clears neg flag"
    )]
    #[test_case(
        0b0000_0001, CPUFlags::empty(), 0b0000_0000, CPUFlags::ZERO | CPUFlags::CARRY;
        "lsr sets carry and zero flag"
    )]
    fn test_lsr(accumulator: u8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.lsr_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.mem_write(0x05, accumulator);
        cpu.mem_write(0x00, 0x05);
        cpu.status = initial_status;
        cpu.lsr(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0101, 0b0000_1100, CPUFlags::empty(), 0b0000_1101, CPUFlags::empty();
        "ora no flags"
    )]
    #[test_case(
        0b0000_0000, 0b0000_0000, CPUFlags::ZERO, 0b0000_0000, CPUFlags::ZERO;
        "ora keeps zero flag"
    )]
    #[test_case(
        0b0000_0001, 0b1000_0001, CPUFlags::empty(), 0b1000_0001, CPUFlags::NEG;
        "ora sets neg flag"
    )]
    fn test_ora(accumulator: u8, operand: u8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, operand);
        cpu.status = initial_status;
        cpu.ora(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0x05, 0x05, CPUFlags::empty();
        "pla no flags"
    )]
    #[test_case(
        0x00, 0x00, CPUFlags::ZERO;
        "pla sets zero flag"
    )]
    #[test_case(
        0x80, 0x80, CPUFlags::NEG;
        "pla sets neg flag"
    )]
    fn test_pla(stack: u8, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.stack_ptr = 0xFE;
        cpu.mem_write(0x01FF, stack);
        cpu.pla();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.stack_ptr, 0xFF);
        assert_eq!(cpu.status, CPUFlags::from_bits_truncate(0b0010_0100) | expected_status);
    }

    #[test_case(
        0b0000_0001, CPUFlags::empty(), 0b0000_0010, CPUFlags::empty();
        "rol no flags"
    )]
    #[test_case(
        0b0000_0001, CPUFlags::CARRY, 0b0000_0011, CPUFlags::empty();
        "rol with carry flag"
    )]
    #[test_case(
        0b1000_0001, CPUFlags::NEG, 0b0000_0010, CPUFlags::CARRY;
        "rol sets carry flag"
    )]
    #[test_case(
        0b1000_0000, CPUFlags::NEG, 0b0000_0000, CPUFlags::ZERO | CPUFlags::CARRY;
        "rol sets zero flag"
    )]
    #[test_case(
        0b0100_0000, CPUFlags::empty(), 0b1000_0000, CPUFlags::NEG;
        "rol sets neg flag"
    )]
    fn test_rol(accumulator: u8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.rol_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.mem_write(0x05, accumulator);
        cpu.mem_write(0x00, 0x05);
        cpu.status = initial_status;
        cpu.rol(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test_case(
        0b0000_0010, CPUFlags::empty(), 0b0000_0001, CPUFlags::empty();
        "ror no flags"
    )]
    #[test_case(
        0b0000_0010, CPUFlags::CARRY, 0b1000_0001, CPUFlags::NEG;
        "ror with carry flag"
    )]
    #[test_case(
        0b1000_0001, CPUFlags::NEG, 0b0100_0000, CPUFlags::CARRY;
        "ror sets carry flag"
    )]
    #[test_case(
        0b0000_0001, CPUFlags::empty(), 0b0000_0000, CPUFlags::ZERO | CPUFlags::CARRY;
        "ror sets zero flag"
    )]
    fn test_ror(accumulator: u8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.status = initial_status;
        cpu.ror_acc();
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);

        cpu.mem_write(0x05, accumulator);
        cpu.mem_write(0x00, 0x05);
        cpu.status = initial_status;
        cpu.ror(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_rti() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.stack_ptr = 0xFC;
        cpu.mem_write(0x01FF, 0x80);
        cpu.mem_write(0x01FE, 0x03);
        cpu.mem_write(0x01FD, (CPUFlags::CARRY | CPUFlags::NEG).bits);
        cpu.rti();
        assert_eq!(cpu.program_counter, 0x8003);
        assert_eq!(cpu.status, CPUFlags::CARRY | CPUFlags::NEG);
    }

    #[test]
    fn test_rts() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.stack_ptr = 0xFD;
        cpu.mem_write(0x01FF, 0x80);
        cpu.mem_write(0x01FE, 0x03);
        cpu.rts();
        assert_eq!(cpu.program_counter, 0x8004);
    }

    #[test_case(
        0x05, 0x01, CPUFlags::empty(), 0x03, CPUFlags::CARRY;
        "sbc no flags"
    )]
    #[test_case(
        0x05, 0x01, CPUFlags::CARRY, 0x04, CPUFlags::CARRY;
        "sbc carry flag set"
    )]
    #[test_case(
        0x05, 0x05, CPUFlags::CARRY, 0x00, CPUFlags::ZERO | CPUFlags::CARRY;
        "sbc sets zero flag"
    )]
    #[test_case(
        0x05, 0x06, CPUFlags::CARRY, 0xFF, CPUFlags::NEG;
        "sbc sets neg flag"
    )]
    #[test_case(
        0x80, 0x01, CPUFlags::CARRY | CPUFlags::NEG, 0x7F, CPUFlags::OVER | CPUFlags::CARRY;
        "sbc sets overflow flag"
    )]
    #[test_case(
        0x00, -0x01, CPUFlags::CARRY, 0x01, CPUFlags::empty();
        "sbc subtracts negative"
    )]
    #[test_case(
        0x7F, -0x01, CPUFlags::CARRY, 0x80, CPUFlags::OVER | CPUFlags::NEG;
        "sbc subtracts negative overflow"
    )]
    fn test_sbc(accumulator: u8, operand: i8, initial_status: CPUFlags, expected_acc: u8, expected_status: CPUFlags) {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = accumulator;
        cpu.mem_write(0x00, operand as u8);
        cpu.status = initial_status;
        cpu.sbc(&AddressingMode::Immediate);
        assert_eq!(cpu.accumulator, expected_acc);
        assert_eq!(cpu.status, expected_status);
    }

    #[test]
    fn test_sta() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.accumulator = 0x01;
        cpu.mem_write(0x00, 0x05);
        cpu.sta(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), 0x01);
    }

    #[test]
    fn test_stx() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_x = 0x01;
        cpu.mem_write(0x00, 0x05);
        cpu.stx(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), 0x01);
    }

    #[test]
    fn test_sty() {
        let bus: Bus = Bus::new(test::test_rom());
        let mut cpu: CPU = CPU::new(bus);
        cpu.register_y = 0x01;
        cpu.mem_write(0x00, 0x05);
        cpu.sty(&AddressingMode::ZeroPage);
        assert_eq!(cpu.mem_read(0x05), 0x01);
    }
}