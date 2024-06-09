const MEM_SIZE: usize = 0xFFFF;
const PRG_START: u16 = 0x8000;

// Flags
const F_NEG: u8 = 0b1000_0000;
const F_OVERFLOW: u8 = 0b0100_0000;
const F_BRK: u8 = 0b0001_0000;
const F_DEC: u8 = 0b0000_1000;
const F_INT: u8 = 0b0000_0100;
const F_ZERO: u8 = 0b0000_0010;
const F_CARRY: u8 = 0b0000_0001;

// Opcodes
const TAX: u8 = 0xAA;
const LDA: u8 = 0xA9;
const INX: u8 = 0xE8;
const BRK: u8 = 0x00;

pub struct CPU {
    pub stack_ptr: u8,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; MEM_SIZE],
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

    fn mem_read(&mut self, addr: u16) -> u8 {
        return self.memory[addr as usize];
    }

    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        let low: u16 = self.mem_read(addr) as u16;
        let high: u16 = self.mem_read(addr + 1) as u16;
        return (high << 8) | low;
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let high: u8 = (data >> 8) as u8;
        let low: u8 = (data & 0xff) as u8;
        self.mem_write(addr, low);
        self.mem_write(addr + 1, high);
    }

    fn set_flag(&mut self, flag: u8) {
        self.status = self.status | flag;
    }
    
    fn unset_flag(&mut self, flag: u8) {
        self.status = self.status & !flag;
    }

    fn set_zero_and_neg_flags(&mut self, val: u8) {
        // Set 0 bit in status accordingly
        if val == 0 {
            self.set_flag(F_ZERO);
        } else {
            self.unset_flag(F_ZERO);
        }

        // Set negative bit in status accordingly
        if val & 0b1000_0000 != 0 {
            self.set_flag(F_NEG);
        }
        else {
            self.unset_flag(F_NEG);
        }
    }

    fn inx(&mut self) {
        if self.register_x == 0xff {
            self.register_x = 0x00;
        }
        else {
            self.register_x += 1;
        }

        self.set_zero_and_neg_flags(self.register_x);
    }

    fn tax(&mut self) {
        self.register_x = self.accumulator;
                    
        self.set_zero_and_neg_flags(self.accumulator);
    }

    fn lda(&mut self) {
        let param: u8 = self.mem_read(self.program_counter);
        self.program_counter += 1;
        self.accumulator = param;

        self.set_zero_and_neg_flags(self.accumulator);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.run();
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[PRG_START as usize .. (PRG_START as usize + program.len())].copy_from_slice(&program[..]);
        self.program_counter = PRG_START;
    }

    pub fn run(&mut self) {
        loop {
            // Get current operation in program
            let opscode: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;

            // Run corresponding operation function
            match opscode {
                TAX => self.tax(),
                INX => self.inx(),
                LDA => self.lda(),
                BRK => { // BRK - end program
                    self.status = self.status | F_BRK;
                    return;
                },
                _ => todo!(""),
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
    fn test_read_memory_u16() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[PRG_START as usize] = 0xAA;
        cpu.memory[(PRG_START + 1) as usize] = 0x05;
        assert_eq!(cpu.mem_read_u16(PRG_START), 0x05AA);
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
    fn test_write_memory_u16() {
        let mut cpu: CPU = CPU::new();
        cpu.mem_write_u16(PRG_START, 0x0508);
        assert_eq!(cpu.memory[PRG_START as usize], 0x08);
        assert_eq!(cpu.memory[(PRG_START + 1) as usize], 0x05);
    }

    #[test]
    fn test_load() {
        let mut cpu: CPU = CPU::new();
        cpu.load(vec![LDA, 0x05, BRK]);
        assert!(cpu.program_counter == PRG_START);
        assert!(cpu.memory[cpu.program_counter as usize] == LDA);
        assert!(cpu.memory[(cpu.program_counter + 1) as usize] == 0x05);
        assert!(cpu.memory[(cpu.program_counter + 2) as usize] == BRK);
    }

    #[test]
    fn test_run() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = PRG_START;
        cpu.memory[PRG_START as usize] = LDA;
        cpu.memory[(PRG_START + 1) as usize] = 0x05;
        cpu.memory[(PRG_START + 2) as usize] = BRK;
        cpu.run();
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.program_counter, PRG_START + 3);
    }

    #[test]
    fn test_load_and_run() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![LDA, 0x05, BRK]);
        assert!(cpu.program_counter == PRG_START + 3);
        assert!(cpu.memory[PRG_START as usize] == LDA);
        assert!(cpu.memory[(PRG_START + 1) as usize] == 0x05);
        assert!(cpu.memory[(PRG_START + 2) as usize] == BRK);
        assert_eq!(cpu.accumulator, 0x05);
    }

    #[test]
    fn test_0x00_brk_break_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![BRK]);
        assert!(cpu.status & F_BRK == F_BRK);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![LDA, 0x05, BRK]);
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![LDA, 0x00, BRK]);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xa9_lda_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![LDA, 0xff, BRK]);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_0xaa_tax_load_data() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0x05;
        cpu.load_and_run(vec![TAX, BRK]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xaa_tax_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0;

        cpu.load_and_run(vec![TAX, BRK]);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xaa_tax_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0xff;

        cpu.load_and_run(vec![TAX, BRK]);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }

    #[test]
    fn test_0xe8_inx_increment() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x00;

        cpu.load_and_run(vec![INX, BRK]);
        assert_eq!(cpu.register_x, 0x01);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0xff;

        cpu.load_and_run(vec![INX, BRK]);
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.status & F_ZERO == F_ZERO);
        assert!(cpu.status & F_NEG == 0);
    }

    #[test]
    fn test_0xe8_inx_negative() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x7F;

        cpu.load_and_run(vec![INX, BRK]);
        assert_eq!(cpu.register_x, 0x80);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);

        cpu.load_and_run(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x81);
        assert!(cpu.status & F_ZERO == 0);
        assert!(cpu.status & F_NEG == F_NEG);
    }
}