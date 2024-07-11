use crate::mem::Mem;

const CPU_START: u16 = 0x0000;
const CPU_END: u16 = 0x1FFF;
const PPU_START: u16 = 0x2000;
const PPU_END: u16 = 0x3FFF;

pub struct Bus {
    ram: [u8; 2048],
}


impl Mem for Bus {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            CPU_START ..= CPU_END => {
                let mirrored_addr: u16 = addr & 0b0111_1111_1111;
                self.ram[mirrored_addr as usize]
            }
            PPU_START ..= PPU_END => {
                let mirrored_addr: u16 = addr & 0x2007;
                0
            }
            _ => {
                0
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        let mirrored_addr: u16 = addr & 0b0111_1111_1111;
        self.ram[mirrored_addr as usize] = data;
    }
}

impl Bus {
    pub fn new() -> Self{
        Bus {
            ram: [0; 2048]
        }
    }
 }