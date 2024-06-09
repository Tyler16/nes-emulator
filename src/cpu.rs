pub struct CPU {
    pub stack_ptr: u8,
    pub accumulator: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
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
        }
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

    fn lda(&mut self, param: u8) {
        self.accumulator = param;

        self.set_zero_and_neg_flags(self.accumulator);
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;

        loop {
            let opscode: u8 = program[self.program_counter as usize];
            self.program_counter += 1;

            match opscode {
                0xAA => self.tax(),
                0xE8 => self.inx(),
                0xA9 => { // LDA
                    // Get and set value for accumulator
                    let param: u8 = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.lda(param);
                },
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
    fn test_0x00_brk_break_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.interpret(vec![0x00]);
        assert!(cpu.status & 0b0001_0000 == 0b0001_0000);
    }

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.interpret(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.accumulator, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.interpret(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.interpret(vec![0xa9, 0xff, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }

    #[test]
    fn test_0xaa_tax_load_data() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0x05;
        cpu.interpret(vec![0xaa, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_zero_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0;

        cpu.interpret(vec![0xaa, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xaa_tax_neg_flag() {
        let mut cpu: CPU = CPU::new();
        cpu.accumulator = 0xff;

        cpu.interpret(vec![0xaa, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }

    #[test]
    fn test_0xe8_inx_increment() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x00;

        cpu.interpret(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x01);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0xff;

        cpu.interpret(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0);
        assert!(cpu.status & 0b0000_0010 == 0b0000_0010);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xe8_inx_negative() {
        let mut cpu: CPU = CPU::new();
        cpu.register_x = 0x7F;

        cpu.interpret(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x80);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);

        cpu.interpret(vec![0xe8, 0x00]);
        assert_eq!(cpu.register_x, 0x81);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0b1000_0000);
    }
}