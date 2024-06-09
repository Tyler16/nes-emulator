pub struct CPU {
    pub stack_ptr: u8,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
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
            memory: [0; 0xFFFF],
        }
    }

    fn mem_read(&mut self, addr: u16) -> u8 {
        return self.memory[addr as usize];
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

    fn set_zero_and_neg_flags(&mut self, val: u8) {
        // Set 0 bit in status accordingly
        if val == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101;
        }

        // Set negative bit in status accordingly
        if val & 0b1000_0000 != 0 {
            self.status = self.status | 0b1000_0000;
        }
        else {
            self.status = self.status & 0b0111_1111;
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
        self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.program_counter = 0x8000;
    }

    pub fn run(&mut self) {
        loop {
            let opscode: u8 = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match opscode {
                0xAA => self.tax(),
                0xE8 => self.inx(),
                0xA9 => self.lda(),
                0x00 => { // BRK - end program
                    self.status = self.status | 0b0001_0000;
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
    fn test_write_memory() {
        let mut cpu: CPU = CPU::new();
        cpu.mem_write(0x8000, 0xA1);
        assert!(cpu.memory[0x8000 as usize] == 0xA1);
        cpu.mem_write(0x8000, 0xA4);
        assert!(cpu.memory[0x8000 as usize] == 0xA4);
    }

    #[test]
    fn test_read_memory() {
        let mut cpu: CPU = CPU::new();
        cpu.memory[0x8000 as usize] = 0xA1;
        assert!(cpu.mem_read(0x8000) == 0xA1);
    }

    #[test]
    fn test_load() {
        let mut cpu: CPU = CPU::new();
        cpu.load(vec![0xa9, 0x05, 0x00]);
        assert!(cpu.program_counter == 0x8000);
        assert!(cpu.memory[cpu.program_counter as usize] == 0xa9);
        assert!(cpu.memory[(cpu.program_counter + 1) as usize] == 0x05);
        assert!(cpu.memory[(cpu.program_counter + 2) as usize] == 0x00);
    }

    #[test]
    fn test_run() {
        let mut cpu: CPU = CPU::new();
        cpu.program_counter = 0x8000;
        cpu.memory[0x8000 as usize] = 0xa9;
        cpu.memory[0x8001 as usize] = 0x05;
        cpu.memory[0x8002 as usize] = 0x00;
        cpu.run();
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.program_counter, 0x8003);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
        assert!(cpu.status & 0b0001_0000 == 0b0001_0000);
    }

    #[test]
    fn test_load_and_run() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert!(cpu.program_counter == 0x8003);
        assert!(cpu.memory[0x8000 as usize] == 0xa9);
        assert!(cpu.memory[0x8001 as usize] == 0x05);
        assert!(cpu.memory[0x8002 as usize] == 0x00);
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
        assert!(cpu.status & 0b0001_0000 == 0b0001_0000);
    }

    #[test]
    fn test_0x00_brk_break_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0x00]);
        assert!(cpu.status & 0b0001_0000 == 0b0001_0000);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }

    #[test]
    fn test_0xaa_tax_load_data() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0x05;
        cpu.load_and_run(vec![0xaa, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0;

        cpu.load_and_run(vec![0xaa, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0xff;

        cpu.load_and_run(vec![0xaa, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }

    #[test]
    fn test_0xe8_inx_increment() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x00;

        cpu.load_and_run(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x01);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0xff;

        cpu.load_and_run(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xe8_inx_negative() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x7F;

        cpu.load_and_run(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x80);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);

        cpu.load_and_run(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x81);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }
}