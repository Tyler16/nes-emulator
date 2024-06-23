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
        OpCode::new(0x29, "AND", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x25, "AND", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x35, "AND", 2, 4, AddressingMode::ZeroPage_X),
        OpCode::new(0x2D, "AND", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x3D, "AND", 3, 4, AddressingMode::Absolute_X),
        OpCode::new(0x39, "AND", 3, 4, AddressingMode::Absolute_Y),
        OpCode::new(0x21, "AND", 2, 6, AddressingMode::Indirect_X),
        OpCode::new(0x31, "AND", 2, 5, AddressingMode::Indirect_Y),

        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::Implicit),

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