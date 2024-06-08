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

    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;

        loop {
            let opscode: u8 = program[self.program_counter as usize];
            self.program_counter += 1;

            match opscode {
                // LDA Immediate - Set accumulator register to a given value
                0xA9 => {
                    // Get and set value for accumulator
                    let param: u8 = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.accumulator = param;

                    // Set 0 bit in status accordingly
                    if self.accumulator == 0 {
                        self.status = self.status | 0b0000_0010;
                    } else {
                        self.status = self.status & 0b1111_1101;
                    }

                    // Set negative bit in status accordingly
                    if self.accumulator & 0b1000_0000 != 0 {
                        self.status = self.status | 0b1000_0000;
                    }
                    else {
                        self.status = self.status & 0b0111_1111;
                    }
                },
                // BRK - Ends program
                0x00 => {
                    self.status = self.status | 0b0001_0000;
                    return;
                },
                _ => todo!(""),
            }
        }
    }
}

fn main() {
    println!("Hello, world!");
}
