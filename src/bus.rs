use crate::mem::Mem;
use crate::rom::Rom;

const CPU_START: u16 = 0x0000;
const CPU_END: u16 = 0x1FFF;
const PPU_START: u16 = 0x2000;
const PPU_END: u16 = 0x3FFF;
const ROM_START: u16 = 0x8000;
const ROM_END: u16 = 0xFFFF;

pub struct Bus {
    ram: [u8; 2048],
    rom: Rom,
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
            ROM_START ..= ROM_END => {
                self.read_prg_rom(addr)
            }
            _ => {
                println!("Ignoring mem access at {}.", addr);
                0
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            CPU_START ..= CPU_END => {
                let mirrored_addr: u16 = addr & 0b0111_1111_1111;
                self.ram[mirrored_addr as usize] = data;
            }
            PPU_START ..= PPU_END => {
                let mirrored_addr: u16 = addr & 0x2007;
            }
            ROM_START ..= ROM_END => {
                panic!("Attempting to write to cartridge space.");
            }
            _ => {
                println!("Ignoring mem write at {}.", addr);
            }
        }
    }
}

impl Bus {
    pub fn new(rom: Rom) -> Self{
        Bus {
            ram: [0; 2048],
            rom: rom,
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.prg.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }
        self.rom.prg[addr as usize]
    }
}