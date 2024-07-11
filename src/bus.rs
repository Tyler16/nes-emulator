use crate::mem::Mem;

pub struct Bus {
    cpu_ram: [u8; 2048],
}

impl Mem for Bus {
    fn mem_read(&mut self, addr: u16) -> u8 {
        
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        
    }
}

impl Bus {
    pub fn new() -> Self{
        Bus {
            cpu_ram: [0; 2048]
        }
    }
 }