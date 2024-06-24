use std::collections::HashMap;
use lazy_static;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Implicit,
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Relative,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect,
    Indirect_X,
    Indirect_Y,
}

pub struct OpCode {
    pub code: u8,
    pub operation: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode,
}

impl OpCode {
    fn new(code: u8, operation: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code: code,
            operation: operation,
            len: len,
            cycles: cycles,
            mode: mode,
        }
    }
}

lazy_static! {
    pub static ref OPCODES: Vec<OpCode> = vec![
        OpCode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x6D, "ADC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7D, "ADC", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x79, "ADC", 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0x61, "ADC", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x71, "ADC", 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x2D, "AND", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3D, "AND", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x39, "AND", 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0x21, "AND", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x31, "AND", 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0x0A, "ASL", 1, 2, AddressingMode::Accumulator),
        OpCode::new(0x06, "ASL", 1, 2, AddressingMode::ZeroPage),
        OpCode::new(0x16, "ASL", 1, 2, AddressingMode::ZeroPage_X),
        OpCode::new(0x0E, "ASL", 1, 2, AddressingMode::Absolute),
        OpCode::new(0x1E, "ASL", 1, 2, AddressingMode::Absolute_X),

        // TODO
        OpCode::new(0x90, "BCC", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0xB0, "BCS", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0xF0, "BEQ", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0x24, "BIT", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x2C, "BIT", 3, 4, AddressingMode::Absolute),

        // TODO
        OpCode::new(0x30, "BMI", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0xD0, "BNE", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0x10, "BPL", 2, 2, AddressingMode::Relative),

        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::Implicit),

        // TODO
        OpCode::new(0x50, "BVC", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0x70, "BVS", 2, 2, AddressingMode::Relative),

        // TODO
        OpCode::new(0x18, "CLC", 1, 2, AddressingMode::Implied),

        // TODO
        OpCode::new(0xD8, "CLD", 1, 2, AddressingMode::Implied),

        // TODO
        OpCode::new(0x58, "CLI", 1, 2, AddressingMode::Implied),

        // TODO
        OpCode::new(0xB8, "CLV", 1, 2, AddressingMode::Implied),

        OpCode::new(0xE8, "INX", 1, 2, AddressingMode::Implied),

        OpCode::new(0xA9, "LDA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xA5, "LDA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xB5, "LDA", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0xAD, "LDA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xBD, "LDA", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0xB9, "LDA", 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0xA1, "LDA", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0xB1, "LDA", 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0xAA, "TAX", 1, 2, AddressingMode::Implied),
    ];

    pub static ref OPCODES_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for op in &*OPCODES {
            map.insert(op.code, op);
        }
        map
    };
}