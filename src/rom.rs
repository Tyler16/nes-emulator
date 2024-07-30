const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_PAGE_SIZE: usize = 16384;
const CHR_PAGE_SIZE: usize = 8192;


pub enum Mirroring {
    VERTICAL,
    HORIZONTAL,
    FOUR_SCREEN,
}

pub struct Rom {
    pub prg: Vec<u8>,
    pub chr: Vec<u8>,
    pub mapper: u8,
    pub mirroring: Mirroring,
}


impl Rom {
    pub fn new(raw: &Vec<u8>) -> Result<Rom, String> {
        if &raw[0..4] != NES_TAG {
            return Err("File is not in correct format.".to_string());
        }

        let mapper: u8 = (raw[6] >> 4) | (raw[7] & 0b1111_0000);

        let ines_ver: u8 = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err("iNES version 2.0 is not supported.".to_string());
        }

        let four_screen: bool = raw[6] & 0b1000 != 0;
        let vertical: bool = raw[6] & 0b1 != 0;
        let mirroring: Mirroring = match (four_screen, vertical) {
            (true, _) => Mirroring::FOUR_SCREEN,
            (false, true) => Mirroring::VERTICAL,
            (false, false) => Mirroring::HORIZONTAL,
        };

        let prg_size: usize = raw[4] as usize * PRG_PAGE_SIZE;
        let chr_size: usize = raw[5] as usize * CHR_PAGE_SIZE;

        let trainer: bool = raw[6] & 0b100 != 0;

        let prg_start: usize = 16 + if trainer {512} else {0};
        let chr_start: usize = prg_start + prg_size;
        Ok(Rom {
            prg: raw[prg_start..(prg_start + prg_size)].to_vec(),
            chr: raw[chr_start..(chr_start + chr_size)].to_vec(),
            mapper: mapper,
            mirroring: mirroring,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_incorrect_format_err() {
        let raw: Vec<u8> = vec![0x00, 0x00, 0x00, 0x00];
        let rom: Result<Rom, String> = Rom::new(&raw);
        assert!(rom.is_err());
        assert_eq!(rom.err().unwrap(), "File is not in correct format.");
    }

    #[test]
    fn test_ines_2_err() {
        let raw: Vec<u8> = vec![0x4E, 0x45, 0x53, 0x1A, 0x00, 0x00, 0x00];
        let rom: Result<Rom, String> = Rom::new(&raw);
        assert!(rom.is_err());
        assert_eq!(rom.err().unwrap(), "File is not in correct format.");
    }
}