const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const PRG_PAGE_SIZE: usize = 16384;
const CHR_PAGE_SIZE: usize = 8192;

#[derive(PartialEq)]
#[allow(non_camel_case_types)]
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
        // Check file format
        if &raw[0..4] != NES_TAG {
            return Err("File is not in correct format.".to_string());
        }

        // Check if version supported
        let ines_ver: u8 = (raw[7] >> 2) & 0b11;
        if ines_ver != 0 {
            return Err("iNES version 2.0 is not supported.".to_string());
        }

        // Get mapping type
        let mapper: u8 = (raw[6] >> 4) | (raw[7] & 0b1111_0000);

        // Check mirroring type
        let four_screen: bool = raw[6] & 0b1000 != 0;
        let vertical: bool = raw[6] & 0b1 != 0;
        let mirroring: Mirroring = match (four_screen, vertical) {
            (true, _) => Mirroring::FOUR_SCREEN,
            (false, true) => Mirroring::VERTICAL,
            (false, false) => Mirroring::HORIZONTAL,
        };

        // Get size of program and graphics data
        let prg_size: usize = raw[4] as usize * PRG_PAGE_SIZE;
        let chr_size: usize = raw[5] as usize * CHR_PAGE_SIZE;

        // Check if trainer section exists
        let trainer: bool = raw[6] & 0b100 != 0;

        // Get start of program and graphics data
        let prg_start: usize = 16 + if trainer {512} else {0};
        let chr_start: usize = prg_start + prg_size;

        // Convert data to ROM
        Ok(Rom {
            prg: raw[prg_start..(prg_start + prg_size)].to_vec(),
            chr: raw[chr_start..(chr_start + chr_size)].to_vec(),
            mapper: mapper,
            mirroring: mirroring,
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use test_case::test_case;

    struct TestRom {
        header: Vec<u8>,
        trainer: Option<Vec<u8>>,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    }


    fn create_rom(rom: TestRom) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::with_capacity(
            rom.header.len()
                + rom.trainer.as_ref().map_or(0, |t| t.len())
                + rom.prg_rom.len()
                + rom.chr_rom.len(),
        );

        result.extend(&rom.header);
        if let Some(t) = rom.trainer {
            result.extend(t);
        }
        result.extend(&rom.prg_rom);
        result.extend(&rom.chr_rom);

        result
    }


    pub fn test_rom() -> Rom {
        let test_rom: Vec<u8> = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg_rom: vec![1; 2 * PRG_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_PAGE_SIZE],
        });

        Rom::new(&test_rom).unwrap()
    }


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

    #[test_case(Mirroring::FOUR_SCREEN;
                "Four Screen")]
    #[test_case(Mirroring::VERTICAL;
                "Vertical")]
    #[test_case(Mirroring::HORIZONTAL;
                "Horizontal")]
    fn test_rom_returned(expected_mirroring: Mirroring) {
        let test_rom: Vec<u8> = create_rom(TestRom {
            header: vec![
                0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 00, 00, 00, 00, 00, 00, 00, 00, 00,
            ],
            trainer: None,
            prg_rom: vec![1; 2 * PRG_PAGE_SIZE],
            chr_rom: vec![2; 1 * CHR_PAGE_SIZE],
        });

        let rom: Rom = Rom::new(&test_rom).unwrap();
        assert!(rom.mirroring == expected_mirroring);
    }
}