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